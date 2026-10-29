// The `pm` API available to pre-request and test scripts. Runs inside a fresh QuickJS context
// that has no file or process access. Input arrives as JSON in `__pmInput`; results leave as
// JSON through `__pmExport()`. The only way out is `__pmSend` (`pm.sendRequest`), which the
// host validates and limits like any other request.
(() => {
  'use strict';

  // Captured before user code runs, so a script cannot swap them out.
  const stringify = JSON.stringify;
  const parse = JSON.parse;
  const input = parse(globalThis.__pmInput);
  const nativeSend = globalThis.__pmSend;
  const nativeDynamic = globalThis.__pmDynamic;
  const nativeXml = globalThis.__pmXml;
  delete globalThis.__pmInput;
  delete globalThis.__pmSend;
  delete globalThis.__pmDynamic;
  delete globalThis.__pmXml;

  const MAX_LOGS = 200;
  const MAX_LOG_CHARS = 2000;
  const MAX_TESTS = 200;
  const MAX_CHANGES = 200;

  const changes = [];
  const tests = [];
  const logs = [];
  const local = {};
  const request = input.request;
  const response = input.response;
  const has = (object, key) => Object.prototype.hasOwnProperty.call(object, key);

  function show(value) {
    if (typeof value === 'string') {
      return value;
    }
    if (value instanceof Error) {
      return `${value.name}: ${value.message}`;
    }
    try {
      const json = JSON.stringify(value);
      return json === undefined ? String(value) : json;
    } catch {
      return String(value);
    }
  }

  // Like `show`, but strings keep their quotes so assertion messages are unambiguous.
  function inspect(value) {
    return typeof value === 'string' ? JSON.stringify(value) : show(value);
  }

  function record(level, args) {
    if (logs.length < MAX_LOGS) {
      logs.push({ level, message: args.map(show).join(' ').slice(0, MAX_LOG_CHARS) });
    }
  }

  // Postman's xml2Json: attributes under "$", text beside child elements under "_".
  globalThis.xml2Json = (text) => {
    const json = nativeXml(String(text));
    if (json === undefined || json === null) {
      throw new Error('xml2Json: the text is not well-formed XML');
    }
    return JSON.parse(json);
  };

  // Latin-1 base64, as in browsers. Postman scripts use these for Basic auth and tokens.
  const BASE64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  globalThis.btoa = (input) => {
    const text = String(input);
    let output = '';
    for (let i = 0; i < text.length; i += 3) {
      const a = text.charCodeAt(i);
      const b = text.charCodeAt(i + 1);
      const c = text.charCodeAt(i + 2);
      if (a > 255 || b > 255 || c > 255) {
        throw new Error('btoa: the string contains characters outside Latin-1');
      }
      const n = (a << 16) | ((b || 0) << 8) | (c || 0);
      output += BASE64[(n >> 18) & 63] + BASE64[(n >> 12) & 63];
      output += i + 1 < text.length ? BASE64[(n >> 6) & 63] : '=';
      output += i + 2 < text.length ? BASE64[n & 63] : '=';
    }
    return output;
  };
  globalThis.atob = (input) => {
    const text = String(input).replace(/=+$/, '');
    let output = '';
    let buffer = 0;
    let bits = 0;
    for (const char of text) {
      const value = BASE64.indexOf(char);
      if (value < 0) {
        throw new Error('atob: the string is not valid base64');
      }
      buffer = (buffer << 6) | value;
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        output += String.fromCharCode((buffer >> bits) & 255);
      }
    }
    return output;
  };

  globalThis.console = {
    log: (...args) => record('log', args),
    info: (...args) => record('info', args),
    debug: (...args) => record('log', args),
    warn: (...args) => record('warn', args),
    error: (...args) => record('error', args),
  };

  // Each change is saved to the database, so a runaway loop must not produce thousands.
  function track(change) {
    if (changes.length < MAX_CHANGES) {
      changes.push(change);
      return true;
    }
    if (changes.length === MAX_CHANGES) {
      record('warn', [`Only the first ${MAX_CHANGES} variable changes of a script are saved`]);
      changes.push(null);
    }
    return false;
  }

  function scope(name) {
    const store = input.variables[name];
    return {
      get: (key) => (has(store, String(key)) ? store[String(key)] : undefined),
      has: (key) => has(store, String(key)),
      set(key, value) {
        const k = String(key);
        const v = show(value);
        if (track({ scope: name, key: k, value: v })) {
          store[k] = v;
        }
      },
      unset(key) {
        const k = String(key);
        if (track({ scope: name, key: k, value: null })) {
          delete store[k];
        }
      },
      toObject: () => ({ ...store }),
    };
  }

  const globals = scope('globals');
  const collectionVariables = scope('collection');
  const environment = scope('environment');
  // A run's data row sits between the pre-request script's own values and the saved scopes.
  const data = input.variables.data;
  const iterationData = {
    get: (key) => (has(data, String(key)) ? data[String(key)] : undefined),
    has: (key) => has(data, String(key)),
    toObject: () => ({ ...data }),
  };
  const layers = [local, data, input.variables.environment, input.variables.collection, input.variables.globals];
  const lookup = (key) =>
    layers.map((layer) => (has(layer, String(key)) ? layer[String(key)] : undefined)).find((v) => v !== undefined);

  function headerList(list) {
    const find = (name) => list.findIndex((h) => h.name.toLowerCase() === String(name).toLowerCase());
    return {
      get: (name) => (find(name) >= 0 ? list[find(name)].value : undefined),
      has: (name) => find(name) >= 0,
      toObject: () => Object.fromEntries(list.map((h) => [h.name, h.value])),
      all: () => list.map((h) => ({ key: h.name, value: h.value })),
      add(header) {
        list.push({ name: String(header.key), value: show(header.value) });
      },
      upsert(header) {
        this.remove(header.key);
        this.add(header);
      },
      remove(name) {
        let index = find(name);
        while (index >= 0) {
          list.splice(index, 1);
          index = find(name);
        }
      },
    };
  }

  class AssertionError extends Error {
    constructor(message) {
      super(message);
      this.name = 'AssertionError';
    }
  }

  function typeOf(value) {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  }

  function deepEqual(a, b) {
    if (Object.is(a, b)) return true;
    if (typeOf(a) !== typeOf(b) || typeof a !== 'object') return false;
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    return aKeys.length === bKeys.length && aKeys.every((k) => has(b, k) && deepEqual(a[k], b[k]));
  }

  class Assertion {
    constructor(value, message) {
      this.value = value;
      this.message = message;
      this.negated = false;
      this.deepMode = false;
    }

    check(passed, expectation) {
      if (this.negated ? passed : !passed) {
        const prefix = this.message ? `${this.message}: ` : '';
        const verb = this.negated ? 'not ' : '';
        throw new AssertionError(`${prefix}expected ${inspect(this.value)} to ${verb}${expectation}`);
      }
      return this;
    }

    get to() { return this; }
    get be() { return this; }
    get been() { return this; }
    get is() { return this; }
    get that() { return this; }
    get which() { return this; }
    get and() { return this; }
    get has() { return this; }
    get have() { return this; }
    get with() { return this; }
    get at() { return this; }
    get of() { return this; }
    get same() { return this; }
    get does() { return this; }
    get not() { this.negated = !this.negated; return this; }
    get deep() { this.deepMode = true; return this; }

    equal(expected) {
      const passed = this.deepMode ? deepEqual(this.value, expected) : this.value === expected;
      return this.check(passed, `equal ${inspect(expected)}`);
    }
    equals(expected) { return this.equal(expected); }
    eq(expected) { return this.equal(expected); }
    eql(expected) { return this.check(deepEqual(this.value, expected), `deeply equal ${inspect(expected)}`); }

    a(type) { return this.check(typeOf(this.value) === String(type).toLowerCase(), `be a ${type}`); }
    an(type) { return this.a(type); }

    include(item) {
      const value = this.value;
      let passed = false;
      if (typeof value === 'string') passed = value.includes(String(item));
      else if (Array.isArray(value)) passed = value.some((v) => (this.deepMode ? deepEqual(v, item) : v === item));
      else if (value && typeof value === 'object' && item && typeof item === 'object')
        passed = Object.keys(item).every((k) => has(value, k) && deepEqual(value[k], item[k]));
      return this.check(passed, `include ${inspect(item)}`);
    }
    includes(item) { return this.include(item); }
    contain(item) { return this.include(item); }
    contains(item) { return this.include(item); }

    property(name, ...expected) {
      const value = this.value;
      const present = value !== null && value !== undefined && Object(value)[name] !== undefined;
      if (expected.length > 0) {
        return this.check(present && deepEqual(value[name], expected[0]), `have property ${inspect(name)} of ${inspect(expected[0])}`);
      }
      this.check(present, `have property ${inspect(name)}`);
      return this.negated ? this : new Assertion(value[name], this.message);
    }

    above(n) { return this.check(this.value > n, `be above ${inspect(n)}`); }
    gt(n) { return this.above(n); }
    below(n) { return this.check(this.value < n, `be below ${inspect(n)}`); }
    lt(n) { return this.below(n); }
    least(n) { return this.check(this.value >= n, `be at least ${inspect(n)}`); }
    gte(n) { return this.least(n); }
    most(n) { return this.check(this.value <= n, `be at most ${inspect(n)}`); }
    lte(n) { return this.most(n); }
    within(low, high) { return this.check(this.value >= low && this.value <= high, `be within ${low}..${high}`); }
    lengthOf(n) { return this.check(this.value != null && this.value.length === n, `have length ${n}`); }
    length(n) { return this.lengthOf(n); }
    match(pattern) { return this.check(pattern.test(String(this.value)), `match ${pattern}`); }
    oneOf(list) { return this.check(list.includes(this.value), `be one of ${inspect(list)}`); }
    keys(...names) {
      const wanted = names.flat();
      const value = this.value ?? {};
      return this.check(wanted.every((k) => has(value, k)), `have keys ${inspect(wanted)}`);
    }

    get ok() { return this.check(Boolean(this.value), 'be truthy'); }
    get true() { return this.check(this.value === true, 'be true'); }
    get false() { return this.check(this.value === false, 'be false'); }
    get null() { return this.check(this.value === null, 'be null'); }
    get undefined() { return this.check(this.value === undefined, 'be undefined'); }
    get exist() { return this.check(this.value !== null && this.value !== undefined, 'exist'); }
    get empty() {
      const value = this.value;
      const size = typeof value === 'string' || Array.isArray(value) ? value.length : Object.keys(value ?? {}).length;
      return this.check(size === 0, 'be empty');
    }
  }

  function makeResponse(response) {
    if (!response) {
      return undefined;
    }
    const code = response.code;
    const statusCheck = (passed, expectation) => {
      if (!passed) {
        throw new AssertionError(`expected response to ${expectation} but got ${code}`);
      }
    };
    return {
      code,
      status: response.status,
      responseTime: response.responseTime,
      responseSize: response.size,
      headers: headerList(response.headers),
      text: () => response.body,
      json: () => parse(response.body),
      to: {
        have: {
          status: (expected) => statusCheck(code === expected, `have status ${expected}`),
          header: (name) => {
            if (!response.headers.some((h) => h.name.toLowerCase() === String(name).toLowerCase())) {
              throw new AssertionError(`expected response to have header ${name}`);
            }
          },
        },
        be: {
          get ok() { statusCheck(code >= 200 && code < 300, 'be ok (2xx)'); return true; },
          get success() { statusCheck(code >= 200 && code < 300, 'be successful (2xx)'); return true; },
          get clientError() { statusCheck(code >= 400 && code < 500, 'be a client error (4xx)'); return true; },
          get serverError() { statusCheck(code >= 500, 'be a server error (5xx)'); return true; },
        },
      },
    };
  }

  function urlText(url) {
    if (url && typeof url === 'object') {
      return String(url.raw ?? url.toString());
    }
    return String(url ?? '');
  }

  // Postman accepts headers as a list of { key, value, disabled }, an object, or "Name: value" lines.
  function headerPairs(headers) {
    if (!headers) {
      return [];
    }
    if (Array.isArray(headers)) {
      return headers
        .filter((h) => h && !h.disabled)
        .map((h) => ({ name: String(h.key ?? h.name), value: show(h.value ?? '') }));
    }
    if (typeof headers === 'string') {
      return headers
        .split(/\r?\n/)
        .map((line) => line.split(':'))
        .filter((parts) => parts.length > 1 && parts[0].trim() !== '')
        .map(([name, ...rest]) => ({ name: name.trim(), value: rest.join(':').trim() }));
    }
    return Object.keys(headers).map((name) => ({ name, value: show(headers[name]) }));
  }

  function setDefaultHeader(headers, name, value) {
    if (!headers.some((h) => h.name.toLowerCase() === name.toLowerCase())) {
      headers.push({ name, value });
    }
  }

  function bodyText(body, headers) {
    if (body === undefined || body === null) {
      return null;
    }
    if (typeof body === 'string') {
      return body;
    }
    const mode = body.mode ?? 'raw';
    if (mode === 'raw') {
      if (body.options?.raw?.language === 'json') {
        setDefaultHeader(headers, 'Content-Type', 'application/json');
      }
      return typeof body.raw === 'string' ? body.raw : show(body.raw ?? '');
    }
    if (mode === 'urlencoded') {
      setDefaultHeader(headers, 'Content-Type', 'application/x-www-form-urlencoded');
      return (body.urlencoded ?? [])
        .filter((pair) => pair && !pair.disabled)
        .map((pair) => `${encodeURIComponent(String(pair.key))}=${encodeURIComponent(show(pair.value ?? ''))}`)
        .join('&');
    }
    throw new Error(`pm.sendRequest does not support "${mode}" bodies yet`);
  }

  // Synchronous under the hood: the callback runs before sendRequest returns.
  function sendRequest(spec, callback) {
    const options = typeof spec === 'string' ? { url: spec } : (spec ?? {});
    const headers = headerPairs(options.header ?? options.headers);
    const outgoing = {
      method: String(options.method ?? 'GET').toUpperCase(),
      url: urlText(options.url),
      headers,
      body: bodyText(options.body, headers),
    };
    const reply = parse(nativeSend(stringify(outgoing)));
    const error = reply.error ? new Error(reply.error) : null;
    const result = reply.response ? makeResponse(reply.response) : undefined;
    if (typeof callback === 'function') {
      callback(error, result);
    } else if (error) {
      throw error;
    }
    return result;
  }

  globalThis.pm = {
    info: {
      eventName: input.phase,
      requestName: input.requestName,
      iteration: input.iteration.index,
      iterationCount: input.iteration.count,
    },
    iterationData,
    globals,
    collectionVariables,
    environment,
    variables: {
      get: lookup,
      replaceIn: (text) =>
        String(text).replace(/\{\{\s*(\$?[\w.-]+)\s*\}\}/g, (match, name) => {
          const value = name.startsWith('$') ? nativeDynamic(name) : lookup(name);
          return value === undefined || value === null ? match : value;
        }),
      has: (key) => layers.some((layer) => has(layer, String(key))),
      set: (key, value) => { local[String(key)] = show(value); },
      unset: (key) => { delete local[String(key)]; },
      toObject: () => Object.assign({}, ...[...layers].reverse()),
    },
    request: {
      get method() { return request.method; },
      set method(value) { request.method = String(value).toUpperCase(); },
      get url() { return request.url; },
      set url(value) { request.url = String(value); },
      headers: headerList(request.headers),
      body: {
        get raw() { return request.body ?? ''; },
        set raw(value) { request.body = show(value); },
      },
    },
    response: makeResponse(response),
    test(name, run) {
      if (tests.length >= MAX_TESTS) return;
      try {
        run();
        tests.push({ name: String(name), passed: true, error: null });
      } catch (error) {
        tests.push({ name: String(name), passed: false, error: show(error && error.message ? error.message : error) });
      }
    },
    expect: (value, message) => new Assertion(value, message),
    sendRequest,
  };

  // Locked down so a script cannot replace how its results are reported.
  Object.defineProperty(globalThis, '__pmExport', {
    value: () =>
      stringify({ changes: changes.filter(Boolean), tests, logs, request, local }),
    writable: false,
    configurable: false,
  });
})();
