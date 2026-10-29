import { autocompletion } from '@codemirror/autocomplete';
import type { Extension } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { graphql, updateSchema } from 'cm6-graphql';
import type { GraphQLSchema } from 'graphql';

/**
 * GraphQL highlighting, plus completion, checking and hover help once a schema is known.
 * This module is loaded on first use, so GraphQL support stays out of the main bundle.
 */
export function graphqlExtension(schema: GraphQLSchema | null): Extension {
  // cm6-graphql offers its suggestions as language data, which the completion popup reads.
  return [graphql(schema ?? undefined), autocompletion()];
}

export function setSchema(view: EditorView, schema: GraphQLSchema | null): void {
  updateSchema(view, schema ?? undefined);
}
