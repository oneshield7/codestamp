import type { GraphQLSchema } from 'graphql';

/** Schemas fetched this session, by endpoint, so reopening a card does not ask again. */
const schemas = new Map<string, GraphQLSchema>();
/** Large schemas take memory; the oldest leave first. */
const MAX_SCHEMAS = 10;

function key(collectionId: string, url: string): string {
  return `${collectionId} ${url.trim()}`;
}

export function knownSchema(collectionId: string, url: string): GraphQLSchema | null {
  return schemas.get(key(collectionId, url)) ?? null;
}

export function rememberSchema(collectionId: string, url: string, schema: GraphQLSchema): void {
  const name = key(collectionId, url);
  schemas.delete(name);
  schemas.set(name, schema);
  const oldest = schemas.keys().next().value;
  if (schemas.size > MAX_SCHEMAS && oldest !== undefined) {
    schemas.delete(oldest);
  }
}
