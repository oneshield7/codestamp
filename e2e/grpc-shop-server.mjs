// A small gRPC server for the e2e checks: the Shop service from protos/shop.proto, with
// server reflection, on a random local port (plaintext).
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const grpc = require('@grpc/grpc-js');
const loader = require('@grpc/proto-loader');
const { ReflectionService } = require('@grpc/reflection');

export const PROTO_DIR = path.join(here, 'protos');

export async function startShop() {
  const definition = loader.loadSync(path.join(PROTO_DIR, 'shop.proto'), {
    includeDirs: [PROTO_DIR],
    longs: String,
    enums: String,
    defaults: true,
  });
  const shop = grpc.loadPackageDefinition(definition).shop;
  const server = new grpc.Server();
  server.addService(shop.Shop.service, {
    GetOrder(call, callback) {
      if (call.request.id === 'missing') {
        callback({ code: grpc.status.NOT_FOUND, details: 'No order missing' });
        return;
      }
      const header = new grpc.Metadata();
      header.set('x-served-by', 'test-shop');
      call.sendMetadata(header);
      const trailer = new grpc.Metadata();
      trailer.set('x-trailer', 'done');
      callback(
        null,
        {
          id: call.request.id,
          total: '1234',
          status: 'PAID',
          items: [{ sku: 'A1', quantity: 2 }],
          auth: String(call.metadata.get('authorization')[0] ?? ''),
          trace: String(call.metadata.get('x-trace')[0] ?? ''),
        },
        trailer,
      );
    },
    ListOrders(call) {
      for (let n = 1; n <= call.request.count; n++) {
        call.write({ id: `o${n}`, total: String(n * 100), status: 'SHIPPED' });
      }
      call.end();
    },
    AddItems(call, callback) {
      let count = 0;
      let quantity = 0;
      call.on('data', (item) => {
        count += 1;
        quantity += item.quantity;
      });
      call.on('end', () => callback(null, { count, quantity }));
    },
    Chat(call) {
      call.on('data', (note) => call.write({ text: `echo: ${note.text}` }));
      call.on('end', () => call.end());
    },
  });
  new ReflectionService(definition).addToServer(server);
  const port = await new Promise((resolve, reject) =>
    server.bindAsync('127.0.0.1:0', grpc.ServerCredentials.createInsecure(), (error, bound) =>
      error ? reject(error) : resolve(bound),
    ),
  );
  return { port, close: () => server.forceShutdown() };
}
