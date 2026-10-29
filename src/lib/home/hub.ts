import type { RequestKind } from '../api/collections';
import { KEEP_DAYS } from '../api/trash';
import { ADDABLE_KINDS } from '../request/kind-icons';
import {
  NAV_ICON_HUES,
  NAV_ICON_PATHS,
  navLabel,
  type NavIconName,
  type NewCollectionStart,
  type SettingsSection,
  type View,
} from '../state/navigation';

/** What a Home button does. Adding requests and Run all need an open collection. */
export type HubAction =
  | { kind: 'newCollection'; start: NewCollectionStart }
  | { kind: 'navigate'; view: View }
  | { kind: 'settings'; section: SettingsSection }
  | { kind: 'addRequest'; request: RequestKind }
  | { kind: 'runAll' }
  | { kind: 'history' }
  | { kind: 'search' }
  | { kind: 'copy' };

export interface HubButton {
  label: string;
  action: HubAction;
}

export interface HubExtra extends HubButton {
  /** SVG path data (24×24). */
  icon: string;
}

/** A command shown with a copy button; several are switched with pills. */
export interface HubCommand {
  label: string;
  command: string;
}

export interface HubTile {
  key: string;
  /** The tile's accessible name and tooltip. */
  name: string;
  /** A `data-hue` name. */
  hue: string;
  icon: string;
  facts: string[];
  title: string;
  description: string;
  commands: HubCommand[];
  primary: HubButton;
  extras: HubExtra[];
}

export interface Hub {
  key: string;
  label: string;
  tiles: HubTile[];
}

const ICONS = {
  plus: 'M12 5v14M5 12h14',
  search: 'M4 11a7 7 0 1 0 14 0a7 7 0 1 0 -14 0M20 20l-4-4',
  join: 'M4 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M4 19a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M16 9a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M6 7v10M18 11c0 3-3 4-6 4H8',
  play: 'M7 5l12 7-12 7z',
  terminal: 'M4 17l6-5-6-5M12 19h8',
  keyboard: 'M3 7h18v10H3zM7 11h.01M11 11h.01M15 11h.01M8 14h8',
} as const;

function navIcon(name: NavIconName): string {
  return NAV_ICON_PATHS.get(name) ?? '';
}

function navHue(name: NavIconName): string {
  return NAV_ICON_HUES.get(name) ?? 'slate';
}

/** What Home says about each kind of request, beyond its name. */
const REQUEST_DETAILS = new Map<RequestKind, { facts: string[]; description: string }>([
  [
    'http',
    {
      facts: ['Any method', 'Path variables', 'Scripts and tests'],
      description:
        'Params, headers, JSON, form data and files, with Bearer, Basic, API key, OAuth 2.0, Digest or AWS Signature auth. Server-Sent Events stream in as they arrive.',
    },
  ],
  [
    'websocket',
    {
      facts: ['Text or JSON', 'Live message log'],
      description:
        'Connect, send text or JSON messages, and read every reply as it arrives in a log you can filter.',
    },
  ],
  [
    'graphql',
    {
      facts: ['Schema completion', 'Variables'],
      description:
        'Write queries with completion from the endpoint’s schema and variables as JSON, with the same auth, scripts and tests as HTTP.',
    },
  ],
  [
    'grpc',
    {
      facts: ['Reflection or .proto', 'Streams'],
      description:
        'Load the services by server reflection or from a .proto file, choose a method and call it: unary, or server, client and bidirectional streams.',
    },
  ],
  [
    'soap',
    {
      facts: ['WSDL operations', 'SOAP 1.1 and 1.2'],
      description:
        'Load a WSDL from a URL or file, pick an operation and send its sample envelope with the right SOAPAction, the way SoapUI does. Responses and faults come back as formatted XML.',
    },
  ],
  [
    'mcp',
    {
      facts: ['STDIO or HTTP', 'Tools, prompts, resources'],
      description:
        'Start an MCP server or reach one over HTTP, then try its tools, prompts and resources with arguments built from their schemas.',
    },
  ],
]);

const EXAMPLE_COMMANDS = new Map<RequestKind, HubCommand[]>([
  ['mcp', [{ label: 'Example server', command: 'npx -y @modelcontextprotocol/server-everything' }]],
]);

const requestTiles: HubTile[] = ADDABLE_KINDS.map((option) => ({
  key: option.kind,
  name: option.name,
  hue: option.hue,
  icon: option.icon,
  facts: REQUEST_DETAILS.get(option.kind)?.facts ?? [],
  title: option.name,
  description: REQUEST_DETAILS.get(option.kind)?.description ?? option.description,
  commands: EXAMPLE_COMMANDS.get(option.kind) ?? [],
  primary: { label: option.label, action: { kind: 'addRequest', request: option.kind } },
  extras: [],
}));

export const HUBS: readonly Hub[] = [
  {
    key: 'start',
    label: 'Start',
    tiles: [
      {
        key: 'new',
        name: 'New collection',
        hue: 'blue',
        icon: ICONS.plus,
        facts: ['Saved on this device', 'Sections and variables'],
        title: 'Create a collection',
        description:
          'Start empty, or give it a base URL that becomes {{baseUrl}} for every request inside. Requests of any kind live in it as numbered steps and save as you type.',
        commands: [],
        primary: { label: 'Create collection', action: { kind: 'newCollection', start: 'empty' } },
        extras: [
          {
            label: 'Import a file instead',
            icon: navIcon('import'),
            action: { kind: 'newCollection', start: 'import' },
          },
        ],
      },
      {
        key: 'import',
        name: navLabel('import'),
        hue: navHue('import'),
        icon: navIcon('import'),
        facts: ['Codestamp', 'Collection v2.1', 'OpenAPI 3', 'Swagger 2', 'WSDL', 'cURL'],
        title: 'Import a collection or cURL',
        description:
          'Bring in a collection or environment file, an API spec or SOAP WSDL from a file or URL, or a cURL command copied from browser dev tools. A preview shows what comes in before anything is saved. Export lives in each collection.',
        commands: [],
        primary: { label: 'Import a file', action: { kind: 'newCollection', start: 'import' } },
        extras: [
          {
            label: 'Paste cURL or an API spec URL',
            icon: ICONS.terminal,
            action: { kind: 'navigate', view: 'import' },
          },
        ],
      },
      {
        key: 'join',
        name: 'Join a shared collection',
        hue: 'violet',
        icon: ICONS.join,
        facts: ['Collaboration over Git', 'No account'],
        title: 'Join a shared collection',
        description:
          'Paste the Git repository address a collaborator shared. Sync now sends your changes and brings in theirs, and overlapping edits are merged field by field. Sign-in goes through Git, so Codestamp never sees a password or token.',
        commands: [],
        primary: { label: 'Join', action: { kind: 'newCollection', start: 'join' } },
        extras: [],
      },
      {
        key: 'search',
        name: 'Quick search',
        hue: 'slate',
        icon: ICONS.search,
        facts: [],
        title: 'Search requests and settings',
        description:
          'Search requests by name, method or path, and collections, environments and Settings pages. Typing a path suggests the rest of it, and Tab completes it.',
        commands: [],
        primary: { label: 'Search', action: { kind: 'search' } },
        extras: [],
      },
    ],
  },
  { key: 'requests', label: 'Requests', tiles: requestTiles },
  {
    key: 'testing',
    label: 'Testing',
    tiles: [
      {
        key: 'run',
        name: 'Run all',
        hue: 'green',
        icon: ICONS.play,
        facts: ['Data files', 'Reports'],
        title: 'Run a whole collection',
        description:
          'Every HTTP and GraphQL request runs in order with its scripts and tests, once or once per row of a CSV or JSON data file. Export the results as a report.',
        commands: [],
        primary: { label: 'Run all', action: { kind: 'runAll' } },
        extras: [],
      },
      {
        key: 'flows',
        name: navLabel('flows'),
        hue: navHue('flows'),
        icon: navIcon('flows'),
        facts: ['Start to End', 'Pass values on', 'Checks'],
        title: 'Run requests in order',
        description:
          'Run requests one after another, such as Register, Login, Add to cart and Checkout. A step can hand a value like a token to the steps after it and check its own response, so you know the API works before you hand it over.',
        commands: [],
        primary: {
          label: `Open ${navLabel('flows')}`,
          action: { kind: 'navigate', view: 'flows' },
        },
        extras: [],
      },
      {
        key: 'catalog',
        name: navLabel('catalog'),
        hue: navHue('catalog'),
        icon: navIcon('catalog'),
        facts: ['Health score', 'Quality rules', 'Owner and tags'],
        title: 'See how each API is doing',
        description:
          'Every collection is listed as an API with a health score built from the requests you sent in the last 7 days, its latest Run all and rules such as secrets kept in variables. Give each API an owner and tags to sort the list.',
        commands: [],
        primary: {
          label: `Open ${navLabel('catalog')}`,
          action: { kind: 'navigate', view: 'catalog' },
        },
        extras: [],
      },
      {
        key: 'cli',
        name: 'Command line',
        hue: 'slate',
        icon: ICONS.terminal,
        facts: ['CI pipelines', 'JUnit and JSON reports'],
        title: 'Run in CI',
        description:
          'codestamp-cli runs a Collaboration folder or a collection file without the app, and exits with 1 when something fails so the pipeline stops.',
        commands: [
          {
            label: 'Folder',
            command: 'codestamp-cli run ./api --environment staging.json --junit report.xml',
          },
          {
            label: 'Collection file',
            command: 'codestamp-cli run users.collection.json --var API_TOKEN',
          },
          { label: 'Data file', command: 'codestamp-cli run ./api --data users.csv --bail' },
        ],
        primary: { label: 'Copy command', action: { kind: 'copy' } },
        extras: [],
      },
    ],
  },
  {
    key: 'workspace',
    label: 'Workspace',
    tiles: [
      {
        key: 'docs',
        name: 'Docs',
        hue: navHue('docs'),
        icon: navIcon('docs'),
        facts: ['Markdown', 'Shared with the collection', 'Save as .md'],
        title: 'Describe each collection and request',
        description:
          'Write what a collection, section or request is for next to what the request sends: params, headers, body and saved examples. Typed-in credentials stay masked, and the docs can be saved as one Markdown file.',
        commands: [],
        primary: { label: 'Open Docs', action: { kind: 'navigate', view: 'docs' } },
        extras: [],
      },
      {
        key: 'changes',
        name: navLabel('changes'),
        hue: navHue('changes'),
        icon: navIcon('changes'),
        facts: ['Commits', 'Field by field', 'Who changed what'],
        title: 'See what changed in shared collections',
        description:
          'Every commit of a shared collection with the requests it added, changed or removed and the fields that differ. The Requests tab shows who changed each request last and how many versions it has.',
        commands: [],
        primary: {
          label: `Open ${navLabel('changes')}`,
          action: { kind: 'navigate', view: 'changes' },
        },
        extras: [],
      },
      {
        key: 'environments',
        name: 'Environments',
        hue: navHue('environments'),
        icon: navIcon('environments'),
        facts: ['Globals', 'Encrypted secrets', 'Dynamic values'],
        title: 'Environments and variables',
        description:
          'Globals, named environments and collection variables. Secret values are encrypted, and {{$guid}} and other dynamic values are filled in when sending.',
        commands: [],
        primary: { label: 'Open Environments', action: { kind: 'navigate', view: 'environments' } },
        extras: [],
      },
      {
        key: 'history',
        name: 'History',
        hue: navHue('history'),
        icon: navIcon('history'),
        facts: ['Per collection', 'Searchable', 'Save back to a collection'],
        title: 'Requests you sent',
        description:
          'Each collection keeps the requests sent from it, under History in its header. Search them, filter them by status, open one again or save it back into a collection. Choose in Settings how long they stay.',
        commands: [],
        primary: { label: 'Open History', action: { kind: 'history' } },
        extras: [],
      },
      {
        key: 'cookies',
        name: 'Cookies',
        hue: navHue('cookies'),
        icon: navIcon('cookies'),
        facts: ['Encrypted', 'Per domain'],
        title: 'Cookies',
        description:
          'Cookies set by responses are kept, sent back to matching hosts like a browser does, and stored encrypted on this device.',
        commands: [],
        primary: { label: 'Open Cookies', action: { kind: 'navigate', view: 'cookies' } },
        extras: [],
      },
      {
        key: 'trash',
        name: navLabel('trash'),
        hue: navHue('trash'),
        icon: navIcon('trash'),
        facts: [`${String(KEEP_DAYS)} days`, 'Undo right after deleting'],
        title: 'Get back what was deleted',
        description: `Deleted collections and requests are kept for ${String(KEEP_DAYS)} days with their examples, Docs, variables and environments. Restore puts them back as they were, and a shared collection syncs again.`,
        commands: [],
        primary: {
          label: `Open ${navLabel('trash')}`,
          action: { kind: 'navigate', view: 'trash' },
        },
        extras: [],
      },
      {
        key: 'settings',
        name: 'Settings',
        hue: navHue('settings'),
        icon: navIcon('settings'),
        facts: ['Themes', 'Fonts', 'Proxy and certificates'],
        title: 'Change how Codestamp looks and connects',
        description:
          'Light, dark or night theme, interface and code fonts, editor behaviour, request defaults, proxy, certificates and backups.',
        commands: [],
        primary: { label: 'Open Settings', action: { kind: 'navigate', view: 'settings' } },
        extras: [
          {
            label: 'Keyboard shortcuts',
            icon: ICONS.keyboard,
            action: { kind: 'settings', section: 'about' },
          },
        ],
      },
    ],
  },
];

/**
 * The tile's main button as it applies now: adding requests and Run all name the open
 * collection, and without one they offer to create it first.
 */
export function primaryFor(tile: HubTile, collectionName: string | null): HubButton {
  const { action } = tile.primary;
  if (action.kind !== 'addRequest' && action.kind !== 'runAll' && action.kind !== 'history') {
    return tile.primary;
  }
  if (collectionName === null) {
    return {
      label: 'Create a collection first',
      action: { kind: 'newCollection', start: 'empty' },
    };
  }
  const labels = {
    runAll: `Run ${collectionName}`,
    history: `Open History of ${collectionName}`,
    addRequest: `${tile.primary.label} to ${collectionName}`,
  };
  const label = labels[action.kind];
  return { label, action };
}
