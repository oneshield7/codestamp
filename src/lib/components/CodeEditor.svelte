<script lang="ts">
  import { Compartment, EditorState } from '@codemirror/state';
  import { EditorView } from '@codemirror/view';
  import type { GraphQLSchema } from 'graphql';
  import { onMount } from 'svelte';
  import {
    baseExtensions,
    languageExtension,
    preferenceExtensions,
    variableHighlight,
    type EditorLanguage,
  } from '../editor/setup';
  import { preferenceSettings } from '../state/preferences.svelte';
  import { VariableHover } from '../state/variable-hover.svelte';
  import { variableNames } from '../state/variable-names.svelte';
  import VariablePopover from './VariablePopover.svelte';

  interface Props {
    value: string;
    label: string;
    language?: EditorLanguage;
    readOnly?: boolean;
    /** Colours `{{name}}` with the names this collection can use. */
    variablesOf?: string | null;
    /** GraphQL: the schema behind completion and checks. */
    schema?: GraphQLSchema | null;
    onChange?: (value: string) => void;
  }

  let {
    value,
    label,
    language = 'text',
    readOnly = false,
    variablesOf,
    schema = null,
    onChange,
  }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined = $state();
  // Only editable text with a collection behind it can show and change variable values.
  const hover = new VariableHover();
  const hoverable = $derived(variablesOf !== undefined && !readOnly);

  $effect(() => () => {
    hover.close();
  });

  $effect(() => {
    if (!hoverable) {
      return;
    }
    const move = (event: PointerEvent): void => {
      if (event.target instanceof Element) {
        hover.point(event.target.closest('[data-variable]'));
      }
    };
    const leave = (): void => {
      hover.point(null);
    };
    host.addEventListener('pointermove', move);
    host.addEventListener('pointerleave', leave);
    return () => {
      host.removeEventListener('pointermove', move);
      host.removeEventListener('pointerleave', leave);
    };
  });
  const languageSlot = new Compartment();
  const preferenceSlot = new Compartment();
  const variableSlot = new Compartment();

  onMount(() => {
    const editor = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          baseExtensions(label, readOnly),
          languageSlot.of(languageExtension(language)),
          preferenceSlot.of(preferenceExtensions(preferenceSettings.editor, readOnly)),
          variableSlot.of([]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              hover.close();
              onChange?.(update.state.doc.toString());
            }
          }),
        ],
      }),
    });
    view = editor;
    return () => {
      editor.destroy();
    };
  });

  $effect(() => {
    const next = value;
    if (view && next !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
    }
  });

  $effect(() => {
    view?.dispatch({ effects: languageSlot.reconfigure(languageExtension(language)) });
  });

  // GraphQL support is a separate download; it is added once loaded, with the current schema.
  let graphqlLoaded = $state(false);
  $effect(() => {
    const editor = view;
    if (language !== 'graphql' || !editor || graphqlLoaded) {
      return;
    }
    void import('../editor/graphql').then(({ graphqlExtension }) => {
      editor.dispatch({ effects: languageSlot.reconfigure(graphqlExtension(schema)) });
      graphqlLoaded = true;
    });
  });
  $effect(() => {
    const editor = view;
    const current = schema;
    if (language === 'graphql' && editor && graphqlLoaded) {
      void import('../editor/graphql').then(({ setSchema }) => {
        setSchema(editor, current);
      });
    }
  });

  // XML and HTML support is a separate download too, swapped in once loaded.
  $effect(() => {
    const editor = view;
    const markup = language;
    if ((markup !== 'xml' && markup !== 'html') || !editor) {
      return;
    }
    const load = { current: true };
    void import('../editor/markup').then(({ markupExtension }) => {
      if (load.current) {
        editor.dispatch({ effects: languageSlot.reconfigure(markupExtension(markup)) });
      }
    });
    return () => {
      load.current = false;
    };
  });

  $effect(() => {
    const extensions = preferenceExtensions(preferenceSettings.editor, readOnly);
    view?.dispatch({ effects: preferenceSlot.reconfigure(extensions) });
  });

  $effect(() => {
    if (variablesOf === undefined) {
      return;
    }
    variableNames.ensure(variablesOf);
    const extensions = variableHighlight(variableNames.known(variablesOf));
    view?.dispatch({ effects: variableSlot.reconfigure(extensions) });
  });
</script>

<div class="editor" bind:this={host}></div>
{#if hover.shown && hoverable}
  <VariablePopover
    shown={hover.shown}
    collectionId={variablesOf ?? null}
    {hover}
    onDismiss={() => view?.focus()}
  />
{/if}

<style>
  .editor {
    height: 100%;
    min-height: 0;
  }
</style>
