// Evaluates the expression given as the first argument in the app page and prints the result.
const pages = await (await fetch('http://127.0.0.1:9333/json')).json();
const socket = new WebSocket(pages.find((p) => p.type === 'page').webSocketDebuggerUrl);
await new Promise((resolve) => socket.addEventListener('open', resolve));
const reply = new Promise((resolve) =>
  socket.addEventListener('message', (event) => resolve(JSON.parse(event.data))),
);
socket.send(
  JSON.stringify({
    id: 1,
    method: 'Runtime.evaluate',
    params: { expression: process.argv[2], awaitPromise: true, returnByValue: true },
  }),
);
const result = await reply;
console.log(
  JSON.stringify(
    result.result?.result?.value ??
      result.result?.exceptionDetails?.exception?.description ??
      result,
    null,
    1,
  ),
);
socket.close();
