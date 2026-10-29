import type { RequestKind } from '../api/collections';

/** How each kind of request is offered for adding: its labels, icon colour and icon. */
export interface AddableKind {
  kind: RequestKind;
  /** The button text, such as "Add request". */
  label: string;
  name: string;
  description: string;
  /** A `data-hue` name. */
  hue: string;
  /** SVG path data on a 24×24 grid. */
  icon: string;
}

export const ADDABLE_KINDS: readonly AddableKind[] = [
  {
    kind: 'http',
    label: 'Add request',
    name: 'HTTP request',
    description: 'Any method, with params, headers, body and auth.',
    hue: 'blue',
    icon: 'M21 3L10 14M21 3l-7 18-4-8-8-4z',
  },
  {
    kind: 'websocket',
    label: 'Add WebSocket',
    name: 'WebSocket',
    description: 'Connect, send messages and read the replies as they arrive.',
    hue: 'teal',
    icon: 'M7 7l-4 5 4 5M17 7l4 5-4 5M10 12h4',
  },
  {
    kind: 'graphql',
    label: 'Add GraphQL',
    name: 'GraphQL',
    description: 'Queries and variables, with completion from the endpoint’s schema.',
    hue: 'rose',
    icon: 'M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9zM12 3L4.2 16.5h15.6z',
  },
  {
    kind: 'grpc',
    label: 'Add gRPC',
    name: 'gRPC',
    description: 'Call methods by server reflection or a .proto file, with streams.',
    hue: 'green',
    icon: 'M4 8h14l-3-3M20 16H6l3 3',
  },
  {
    kind: 'mcp',
    label: 'Add MCP server',
    name: 'MCP server',
    description: 'Start or reach a server, then try its tools, prompts and resources.',
    hue: 'amber',
    icon: 'M5 12h4l2-6 3 12 2-6h3',
  },
  {
    kind: 'soap',
    label: 'Add SOAP',
    name: 'SOAP',
    description: 'Load a WSDL, pick an operation and send its envelope, like SoapUI.',
    hue: 'violet',
    icon: 'M4 6h16v12H4zM4 7l8 6 8-6',
  },
];
