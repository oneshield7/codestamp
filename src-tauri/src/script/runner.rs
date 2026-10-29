use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{Context, Ctx, Function, Runtime};

use super::models::{ScriptInput, ScriptOutput};
use super::network::{Network, failure_json};
use crate::environment::dynamic;
use crate::error::AppError;
use crate::soap::xml_to_json;

const PRELUDE: &str = include_str!("pm.js");
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const STACK_LIMIT_BYTES: usize = 512 * 1024;
const TIME_LIMIT: Duration = Duration::from_secs(2);
/// Reading the error and the results can run script code too (getters, `toJSON`), so that is
/// time-limited as well.
const READ_LIMIT: Duration = Duration::from_millis(500);
const MAX_SEND_REQUESTS: u32 = 10;

/// What a script run produced. `output` is still filled when the script itself threw, so
/// tests and logs written before the error are not lost.
#[derive(Debug)]
pub struct RunResult {
    pub output: ScriptOutput,
    pub error: Option<String>,
}

/// Runs one script in a brand-new QuickJS runtime that is dropped afterwards. The runtime has
/// no file or process access, a memory and stack cap, and a hard time limit. The network is
/// only reachable through `pm.sendRequest` when `network` is given.
/// Blocking: call it from a blocking thread.
pub fn run(
    script: &str,
    input: &ScriptInput,
    network: Option<Network>,
) -> Result<RunResult, AppError> {
    let input_json = serde_json::to_string(input).map_err(|_| engine_error())?;
    let runtime = Runtime::new().map_err(|_| engine_error())?;
    runtime.set_memory_limit(MEMORY_LIMIT_BYTES);
    runtime.set_max_stack_size(STACK_LIMIT_BYTES);

    // The limit applies to the user's script only, so results can still be read afterwards.
    let enforce = Rc::new(Cell::new(false));
    let timed_out = Rc::new(Cell::new(false));
    let deadline = Rc::new(Cell::new(Instant::now()));
    {
        let (enforce, timed_out, deadline) = (enforce.clone(), timed_out.clone(), deadline.clone());
        runtime.set_interrupt_handler(Some(Box::new(move || {
            let stop = enforce.get() && Instant::now() > deadline.get();
            if stop {
                timed_out.set(true);
            }
            stop
        })));
    }

    let context = Context::full(&runtime).map_err(|_| engine_error())?;
    context.with(|ctx| {
        ctx.globals()
            .set("__pmInput", input_json)
            .map_err(|_| engine_error())?;
        let send = send_function(&ctx, network, deadline.clone()).map_err(|_| engine_error())?;
        ctx.globals()
            .set("__pmSend", send)
            .map_err(|_| engine_error())?;
        // `pm.variables.replaceIn('{{$guid}}')` uses the same generator as requests.
        let generate = Function::new(ctx.clone(), |name: String| dynamic::generate(&name))
            .map_err(|_| engine_error())?;
        ctx.globals()
            .set("__pmDynamic", generate)
            .map_err(|_| engine_error())?;
        // `xml2Json` (as in Postman) reads XML, such as SOAP responses, with the core's parser.
        let xml = Function::new(ctx.clone(), |text: String| {
            xml_to_json(&text).ok().map(|value| value.to_string())
        })
        .map_err(|_| engine_error())?;
        ctx.globals()
            .set("__pmXml", xml)
            .map_err(|_| engine_error())?;
        ctx.eval::<(), _>(PRELUDE).map_err(|_| engine_error())?;

        deadline.set(Instant::now() + TIME_LIMIT);
        enforce.set(true);
        let outcome = ctx.eval::<(), _>(script);
        let script_timed_out = timed_out.get();
        deadline.set(Instant::now() + READ_LIMIT);

        let error = match outcome {
            Ok(()) => None,
            Err(_) if script_timed_out => Some(format!(
                "The script was stopped after {} seconds",
                TIME_LIMIT.as_secs()
            )),
            Err(rquickjs::Error::Exception) => Some(describe_exception(&ctx)),
            Err(other) => Some(other.to_string()),
        };
        let exported = ctx.eval::<String, _>("__pmExport()");
        enforce.set(false);
        let output: ScriptOutput =
            serde_json::from_str(&exported.map_err(|_| unreadable())?).map_err(|_| unreadable())?;
        Ok(RunResult { output, error })
    })
}

/// `pm.sendRequest` calls this with the request as JSON and gets the response as JSON.
fn send_function<'js>(
    ctx: &Ctx<'js>,
    network: Option<Network>,
    deadline: Rc<Cell<Instant>>,
) -> rquickjs::Result<Function<'js>> {
    let calls = Cell::new(0_u32);
    Function::new(ctx.clone(), move |request: String| -> String {
        let Some(network) = &network else {
            return failure_json("pm.sendRequest is not available here");
        };
        if calls.get() >= MAX_SEND_REQUESTS {
            return failure_json("A script can send up to 10 requests");
        }
        calls.set(calls.get() + 1);
        let started = Instant::now();
        let reply = network.send(&request);
        // Waiting for the network does not count towards the script's time limit.
        deadline.set(deadline.get() + started.elapsed());
        reply
    })
}

fn describe_exception(ctx: &Ctx<'_>) -> String {
    let thrown = ctx.catch();
    if let Some(exception) = thrown.as_exception() {
        let name: Option<String> = exception.get("name").ok();
        let message = exception.message().unwrap_or_default();
        return match name {
            Some(name) if !name.is_empty() => format!("{name}: {message}"),
            _ => message,
        };
    }
    thrown
        .as_string()
        .and_then(|text| text.to_string().ok())
        .unwrap_or_else(|| "The script threw a value that is not an error".to_owned())
}

fn engine_error() -> AppError {
    AppError::Script("The script engine could not run this script".to_owned())
}

fn unreadable() -> AppError {
    AppError::Script(
        "The script's results could not be read; it may have changed built-in objects".to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::request::types::{Iteration, NameValue};
    use crate::script::models::{
        ChangeScope, Phase, ScriptRequest, ScriptResponse, VariableSnapshot,
    };

    fn input(response: Option<ScriptResponse>) -> ScriptInput {
        ScriptInput {
            phase: if response.is_some() {
                Phase::Test
            } else {
                Phase::PreRequest
            },
            request_name: "List users".to_owned(),
            variables: VariableSnapshot {
                globals: HashMap::from([("region".to_owned(), "id".to_owned())]),
                collection: HashMap::new(),
                environment: HashMap::from([("token".to_owned(), "abc".to_owned())]),
                data: HashMap::from([("email".to_owned(), "ada@example.test".to_owned())]),
            },
            request: ScriptRequest {
                method: "GET".to_owned(),
                url: "{{baseUrl}}/users".to_owned(),
                headers: vec![NameValue {
                    name: "Accept".to_owned(),
                    value: "application/json".to_owned(),
                }],
                body: None,
            },
            response,
            iteration: Iteration { index: 1, count: 3 },
        }
    }

    fn response(code: u16, body: &str) -> ScriptResponse {
        ScriptResponse {
            code,
            status: "OK".to_owned(),
            response_time: 12,
            size: 20,
            headers: vec![NameValue {
                name: "Content-Type".to_owned(),
                value: "application/json".to_owned(),
            }],
            body: body.to_owned(),
        }
    }

    #[test]
    fn pre_request_script_reads_and_writes_variables_and_the_request() {
        let script = r"
            const token = pm.environment.get('token');
            pm.request.headers.upsert({ key: 'Authorization', value: 'Bearer ' + token });
            pm.request.url = pm.request.url + '?page=2';
            pm.environment.set('lastRun', 42);
            console.log('region was', pm.variables.get('region'));
            pm.globals.unset('region');
            pm.variables.set('local', 'yes');
        ";
        let result = run(script, &input(None), None).expect("run");

        assert_eq!(result.error, None);
        let out = result.output;
        assert!(out.request.headers.iter().any(|h| h.value == "Bearer abc"));
        assert_eq!(out.request.url, "{{baseUrl}}/users?page=2");
        assert_eq!(out.local.get("local").map(String::as_str), Some("yes"));
        assert!(
            out.changes
                .iter()
                .any(|c| c.scope == ChangeScope::Environment
                    && c.key == "lastRun"
                    && c.value.as_deref() == Some("42"))
        );
        assert!(
            out.changes
                .iter()
                .any(|c| c.scope == ChangeScope::Globals && c.value.is_none())
        );
        assert_eq!(
            out.logs.first().map(|l| l.message.as_str()),
            Some("region was id")
        );
    }

    #[test]
    fn test_script_reports_passing_and_failing_tests() {
        let script = r#"
            pm.test('status is 200', () => pm.response.to.have.status(200));
            pm.test('body has users', () => {
                const body = pm.response.json();
                pm.expect(body).to.have.property('users').that.is.an('array').with.lengthOf(2);
                pm.expect(body.users[0].name).to.equal('Rina');
            });
            pm.test('fails clearly', () => pm.expect(pm.response.code).to.be.below(100));
            pm.test('not and include', () => pm.expect('hello').to.not.include('bye'));
        "#;
        let body = r#"{"users":[{"name":"Rina"},{"name":"Budi"}]}"#;
        let result = run(script, &input(Some(response(200, body))), None).expect("run");

        let tests = result.output.tests;
        assert_eq!(tests.len(), 4);
        assert_eq!(tests.iter().filter(|t| t.passed).count(), 3);
        let failed = tests.iter().find(|t| !t.passed).expect("one failure");
        assert_eq!(failed.name, "fails clearly");
        assert!(
            failed
                .error
                .as_deref()
                .is_some_and(|e| e.contains("expected 200 to be below 100"))
        );
    }

    #[test]
    fn base64_helpers_and_replace_in_match_postman() {
        let script = r#"
            pm.test('btoa', () => pm.expect(btoa('user:p@ss')).to.equal('dXNlcjpwQHNz'));
            pm.test('atob', () => pm.expect(atob('Zm8=')).to.equal('fo'));
            pm.test('replaceIn', () =>
                pm.expect(pm.variables.replaceIn('{{token}}-{{ region }}-{{missing}}'))
                    .to.equal('abc-id-{{missing}}'));
            pm.test('dynamic', () => {
                const id = pm.variables.replaceIn('{{$guid}}');
                pm.expect(id).to.have.lengthOf(36);
                pm.expect(pm.variables.replaceIn('{{$guid}}')).to.not.equal(id);
                pm.expect(pm.variables.replaceIn('{{$unknownThing}}')).to.equal('{{$unknownThing}}');
                pm.expect(typeof __pmDynamic).to.equal('undefined');
            });
        "#;
        let result = run(script, &input(None), None).expect("run");
        assert_eq!(result.error, None);
        assert!(
            result.output.tests.iter().all(|t| t.passed),
            "{:?}",
            result.output.tests
        );
    }

    #[test]
    fn xml2json_reads_soap_responses_like_postman() {
        let script = r#"
            const body = xml2Json(pm.response.text());
            const result = body['soap:Envelope']['soap:Body'].AddResponse;
            pm.test('reads elements and attributes', () => {
                pm.expect(result.AddResult).to.equal('5');
                pm.expect(result.$.xmlns).to.equal('http://tempuri.org/');
            });
            pm.test('refuses broken XML', () => {
                let refused = false;
                try { xml2Json('<a>'); } catch (error) { refused = true; }
                pm.expect(refused).to.equal(true);
            });
            pm.test('keeps the bridge private', () =>
                pm.expect(typeof __pmXml).to.equal('undefined'));
        "#;
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><AddResponse xmlns="http://tempuri.org/"><AddResult>5</AddResult></AddResponse></soap:Body></soap:Envelope>"#;
        let result = run(script, &input(Some(response(200, body))), None).expect("run");
        assert_eq!(result.error, None);
        assert_eq!(result.output.tests.len(), 3);
        assert!(
            result.output.tests.iter().all(|t| t.passed),
            "{:?}",
            result.output.tests
        );
    }

    #[test]
    fn script_errors_keep_earlier_results() {
        let script = "pm.test('first', () => {}); console.log('before'); undefinedFunction();";
        let result = run(script, &input(None), None).expect("run");
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|e| e.contains("ReferenceError"))
        );
        assert_eq!(result.output.tests.len(), 1);
        assert_eq!(result.output.logs.len(), 1);
    }

    #[test]
    fn infinite_loops_are_stopped() {
        let started = Instant::now();
        let result = run("while (true) {}", &input(None), None).expect("run");
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|e| e.contains("stopped"))
        );
        assert!(started.elapsed() < TIME_LIMIT + Duration::from_secs(2));
    }

    #[test]
    fn memory_hogs_are_stopped() {
        let result = run(
            "const parts = []; while (true) { parts.push('x'.repeat(1024 * 1024)); }",
            &input(None),
            None,
        )
        .expect("run");
        assert!(result.error.is_some());
    }

    /// Answers each request with JSON describing what it received.
    fn echo_server(requests: usize) -> String {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        std::thread::spawn(move || {
            for stream in listener.incoming().take(requests) {
                let mut stream = stream.expect("stream");
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut line = String::new();
                reader.read_line(&mut line).expect("request line");
                let method = line.split(' ').next().unwrap_or_default().to_owned();
                let (mut trace, mut length) = (String::new(), 0_usize);
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).expect("header");
                    let header = header.trim_end();
                    if header.is_empty() {
                        break;
                    }
                    let (name, value) = header.split_once(':').unwrap_or_default();
                    match name.to_ascii_lowercase().as_str() {
                        "x-trace" => value.trim().clone_into(&mut trace),
                        "content-length" => length = value.trim().parse().unwrap_or(0),
                        _ => {}
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).expect("body");
                let reply = serde_json::json!({
                    "method": method,
                    "trace": trace,
                    "body": String::from_utf8_lossy(&body),
                })
                .to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                )
                .expect("reply");
            }
        });
        format!("http://{address}")
    }

    fn network() -> (tokio::runtime::Runtime, Network) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime");
        let client = crate::request::client::HttpClient::new().expect("client");
        let network = Network::new(client, runtime.handle().clone());
        (runtime, network)
    }

    #[test]
    fn send_request_reaches_the_network_and_returns_the_response() {
        let base = echo_server(1);
        let (_runtime, network) = network();
        let script = format!(
            r#"
            pm.sendRequest({{
                url: '{base}/login',
                method: 'post',
                header: [{{ key: 'X-Trace', value: 't1' }}, {{ key: 'X-Off', value: 'no', disabled: true }}],
                body: {{ mode: 'raw', raw: '{{"a":1}}', options: {{ raw: {{ language: 'json' }} }} }},
            }}, (error, res) => {{
                pm.test('no error', () => pm.expect(error).to.equal(null));
                pm.test('echoed', () => {{
                    const body = res.json();
                    pm.expect(res.code).to.equal(200);
                    pm.expect(body.method).to.equal('POST');
                    pm.expect(body.trace).to.equal('t1');
                    pm.expect(body.body).to.equal('{{"a":1}}');
                }});
                pm.environment.set('fromSend', res.json().method);
            }});
            "#
        );
        let result = run(&script, &input(None), Some(network)).expect("run");
        assert_eq!(result.error, None);
        assert!(
            result.output.tests.iter().all(|t| t.passed) && result.output.tests.len() == 2,
            "{:?}",
            result.output.tests
        );
        assert!(
            result
                .output
                .changes
                .iter()
                .any(|c| c.key == "fromSend" && c.value.as_deref() == Some("POST"))
        );
    }

    #[test]
    fn send_request_is_validated_and_limited() {
        let (_runtime, network) = network();
        let script = r"
            let last = '';
            pm.sendRequest('file:///C:/Windows/win.ini', (error) => {
                pm.test('scheme refused', () => pm.expect(error.message).to.include('http://'));
            });
            for (let i = 0; i < 10; i++) pm.sendRequest('ftp://example.com', (error) => { last = error.message; });
            pm.test('limited to 10', () => pm.expect(last).to.include('up to 10'));
            pm.test('no callback throws', () => {
                let thrown = null;
                try { pm.sendRequest('ftp://example.com'); } catch (error) { thrown = error; }
                pm.expect(thrown).to.not.equal(null);
            });
        ";
        let result = run(script, &input(None), Some(network)).expect("run");
        assert_eq!(result.error, None);
        assert!(
            result.output.tests.iter().all(|t| t.passed),
            "{:?}",
            result.output.tests
        );
        let without_network = run(
            "pm.sendRequest('https://example.com', (e) => pm.test('off', () => pm.expect(e.message).to.include('not available')));",
            &input(None),
            None,
        )
        .expect("run");
        assert!(without_network.output.tests.iter().all(|t| t.passed));
    }

    #[test]
    fn variable_changes_are_capped() {
        let script = "for (let i = 0; i < 500; i++) pm.environment.set('k' + i, i);";
        let result = run(script, &input(None), None).expect("run");
        assert_eq!(result.output.changes.len(), 200);
        assert!(result.output.logs.iter().any(|l| l.level == "warn"));
    }

    #[test]
    fn reading_results_cannot_hang_the_engine() {
        let started = Instant::now();
        let hijacked = run(
            "Object.prototype.toJSON = function () { for (;;) {} };",
            &input(None),
            None,
        );
        assert!(hijacked.is_err());
        let getter = run(
            "throw Object.defineProperty(new Error('boom'), 'name', { get() { for (;;) {} } });",
            &input(None),
            None,
        )
        .expect("run");
        assert!(getter.error.is_some());
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn the_sandbox_has_no_host_access_and_cannot_hijack_results() {
        let script = r"
            pm.test('no require', () => pm.expect(typeof require).to.equal('undefined'));
            pm.test('no std/os', () => pm.expect(typeof std === 'undefined' && typeof os === 'undefined').to.be.true);
            pm.test('no fetch', () => pm.expect(typeof fetch).to.equal('undefined'));
            try { globalThis.__pmExport = () => '{}'; } catch (e) {}
        ";
        let result = run(script, &input(None), None).expect("run");
        assert_eq!(result.error, None);
        assert_eq!(result.output.tests.len(), 3);
        assert!(result.output.tests.iter().all(|t| t.passed));
    }
}
