// A small MCP server for the end-to-end checks. `node mcp-server.mjs stdio` speaks JSON-RPC over
// stdin/stdout; `node mcp-server.mjs http <port>` serves Streamable HTTP on /mcp (tools/call is
// answered as an event stream, everything else as JSON). Only the parts Codestamp uses are here.
import http from 'node:http';
import readline from 'node:readline';
import { randomUUID } from 'node:crypto';

const TOOLS = [
  {
    name: 'echo',
    description: 'Returns the text it was given',
    inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
  },
  {
    name: 'add',
    description: 'Adds two numbers',
    inputSchema: {
      type: 'object',
      properties: { a: { type: 'number' }, b: { type: 'number', default: 2 } },
    },
  },
  {
    name: 'whoami',
    description: 'Shows CODESTAMP_TEST_TOKEN from the environment',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'fail',
    description: 'Always reports a tool error',
    inputSchema: { type: 'object', properties: {} },
  },
];
const PROMPTS = [
  { name: 'greet', description: 'Greets someone', arguments: [{ name: 'name', required: true }] },
];
const RESOURCES = [{ uri: 'note://readme', name: 'Read me', mimeType: 'text/plain' }];

/** The answer to one request, plus notifications to send before it. */
function answer(message) {
  const { id, method, params = {} } = message;
  const ok = (result, notes = []) => ({ notes, reply: { jsonrpc: '2.0', id, result } });
  const fail = (code, text) => ({
    notes: [],
    reply: { jsonrpc: '2.0', id, error: { code, message: text } },
  });
  switch (method) {
    case 'initialize':
      return ok({
        protocolVersion: params.protocolVersion ?? '2025-06-18',
        serverInfo: { name: 'codestamp-test', version: '1.0.0' },
        capabilities: { tools: {}, prompts: {}, resources: {}, logging: {} },
        instructions: 'Test server for Codestamp.',
      });
    case 'ping':
      return ok({});
    case 'tools/list':
      return ok({ tools: TOOLS });
    case 'prompts/list':
      return ok({ prompts: PROMPTS });
    case 'resources/list':
      return ok({ resources: RESOURCES });
    case 'tools/call': {
      const args = params.arguments ?? {};
      const log = {
        jsonrpc: '2.0',
        method: 'notifications/message',
        params: { level: 'info', data: `calling ${params.name}` },
      };
      switch (params.name) {
        case 'echo':
          return ok({ content: [{ type: 'text', text: String(args.text ?? '') }] }, [log]);
        case 'add':
          return ok(
            {
              content: [{ type: 'text', text: String(Number(args.a) + Number(args.b)) }],
              structuredContent: { sum: Number(args.a) + Number(args.b) },
            },
            [log],
          );
        case 'whoami':
          return ok(
            { content: [{ type: 'text', text: process.env.CODESTAMP_TEST_TOKEN ?? '(none)' }] },
            [log],
          );
        case 'fail':
          return ok(
            { content: [{ type: 'text', text: 'This tool always fails' }], isError: true },
            [log],
          );
        default:
          return fail(-32602, `Unknown tool: ${params.name}`);
      }
    }
    case 'prompts/get':
      return ok({
        messages: [
          {
            role: 'user',
            content: { type: 'text', text: `Hello, ${params.arguments?.name ?? 'stranger'}!` },
          },
        ],
      });
    case 'resources/read':
      return params.uri === 'note://readme'
        ? ok({
            contents: [
              { uri: 'note://readme', mimeType: 'text/plain', text: 'Codestamp reads resources.' },
            ],
          })
        : fail(-32002, 'Resource not found');
    default:
      return fail(-32601, `Method not found: ${method}`);
  }
}

if (process.argv[2] === 'stdio') {
  process.stderr.write('codestamp-test ready\n');
  const lines = readline.createInterface({ input: process.stdin });
  lines.on('line', (line) => {
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      process.stderr.write(`not json: ${line}\n`);
      return;
    }
    if (message.id === undefined) return;
    const { notes, reply } = answer(message);
    for (const note of notes) process.stdout.write(`${JSON.stringify(note)}\n`);
    process.stdout.write(`${JSON.stringify(reply)}\n`);
  });
} else {
  const port = Number(process.argv[3] ?? 0);
  const sessions = new Set();
  const server = http.createServer((request, response) => {
    if (!request.url?.startsWith('/mcp')) {
      response.writeHead(404).end();
      return;
    }
    const session = request.headers['mcp-session-id'];
    if (request.method === 'DELETE') {
      sessions.delete(session);
      process.stdout.write(`deleted ${session}\n`);
      response.writeHead(200).end();
      return;
    }
    if (request.method !== 'POST') {
      response.writeHead(405).end();
      return;
    }
    let body = '';
    request.on('data', (chunk) => (body += chunk));
    request.on('end', () => {
      const message = JSON.parse(body);
      if (message.method !== 'initialize' && !sessions.has(session)) {
        response.writeHead(404, { 'content-type': 'text/plain' }).end('Unknown session');
        return;
      }
      if (message.id === undefined) {
        response.writeHead(202).end();
        return;
      }
      if (request.headers['x-test'] !== 'abc') {
        response.writeHead(401, { 'content-type': 'text/plain' }).end('Missing X-Test header');
        return;
      }
      const { notes, reply } = answer(message);
      if (message.method === 'initialize') {
        const id = randomUUID();
        sessions.add(id);
        response
          .writeHead(200, { 'content-type': 'application/json', 'mcp-session-id': id })
          .end(JSON.stringify(reply));
      } else if (message.method === 'tools/call') {
        response.writeHead(200, { 'content-type': 'text/event-stream' });
        for (const note of notes)
          response.write(`event: message\ndata: ${JSON.stringify(note)}\n\n`);
        response.end(`event: message\ndata: ${JSON.stringify(reply)}\n\n`);
      } else {
        response.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(reply));
      }
    });
  });
  server.listen(port, '127.0.0.1', () => {
    process.stdout.write(`listening ${server.address().port}\n`);
  });
}
