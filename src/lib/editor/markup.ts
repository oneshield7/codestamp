import { html } from '@codemirror/lang-html';
import { xml } from '@codemirror/lang-xml';
import type { Extension } from '@codemirror/state';
import type { MarkupLanguage } from '../format/markup';

/** XML and HTML highlighting. This module is loaded on first use, outside the main bundle. */
export function markupExtension(language: MarkupLanguage): Extension {
  return language === 'xml' ? xml() : html();
}
