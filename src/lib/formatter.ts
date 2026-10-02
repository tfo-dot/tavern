import { marked } from 'marked';
import DOMPurify from 'dompurify';
import type { RegexRule } from './types';

// Configure marked options
marked.setOptions({
  gfm: true,
  breaks: true,
});

/**
 * Applies user-defined regex display rules to text before or during formatting.
 */
export function applyRegexRules(text: string, rules: RegexRule[] = []): string {
  if (!text || !rules || rules.length === 0) return text;
  let result = text;
  for (const rule of rules) {
    if (rule.enabled && rule.run_on_display && rule.pattern) {
      try {
        const flags = rule.case_insensitive ? 'gi' : 'g';
        const re = new RegExp(rule.pattern, flags);
        result = result.replace(re, rule.replacement);
      } catch (e) {
        console.warn('Invalid regex rule pattern:', rule.pattern, e);
      }
    }
  }
  return result;
}

/**
 * Formats a message content string into styled HTML:
 * - Macro interpolation ({{char}}, {{user}}, {char}, {user}, etc.)
 * - Markdown parsing (bold, lists, tables, code, images, links, blockquotes)
 * - Roleplay action text styling (*actions* rendered in italics without asterisks)
 * - Spoken quotes styling ("dialogue" rendered in high-contrast color)
 * - Scaled images/GIFs with click preview hooks
 * - Sanitized via DOMPurify
 */
export function formatMessageContent(
  rawText: string,
  charName = 'Character',
  userName = 'User',
  regexRules: RegexRule[] = [],
): string {
  if (!rawText) return '';

  let text = rawText
    .replace(/\{\{char\}\}/gi, charName)
    .replace(/\{\{user\}\}/gi, userName)
    .replace(/\{char\}/gi, charName)
    .replace(/\{user\}/gi, userName)

  text = applyRegexRules(text, regexRules);

  text = text.replace(
    /!\[([^\]]*)\]\(([^)]+)\)/g,
    '<img src="$2" alt="$1" loading="lazy" />',
  );

  let html = marked.parse(text) as string;

  html = html.replace(
    /!\[([^\]]*)\]\(([^)]+)\)/g,
    '<img src="$2" alt="$1" loading="lazy" />',
  );

  html = html.replace(
    /&quot;([^<>\n]+?)&quot;/g,
    '<span class="rp-speech">&quot;$1&quot;</span>',
  );
  html = html.replace(
    /&#8220;([^<>\n]+?)&#8221;/g,
    '<span class="rp-speech">&#8220;$1&#8221;</span>',
  );
  html = html.replace(
    /“([^<>\n]+?)”/g,
    '<span class="rp-speech">“$1”</span>',
  );
  html = html.replace(
    /「([^<>\n]+?)」/g,
    '<span class="rp-speech">「$1」</span>',
  );
  html = html.replace(
    /«([^<>\n]+?)»/g,
    '<span class="rp-speech">«$1»</span>',
  );

  html = html.replace(
    /<em>(.*?)<\/em>/gs,
    '<span class="rp-action">$1</span>',
  );

  return DOMPurify.sanitize(html, SANITIZE_CFG);
}

/**
 * Formats Creator Notes with full HTML/CSS support.
 * Allows safe styles, keyframes, grids, and tables while blocking scripts.
 */
export function formatCreatorNotes(rawContent: string): string {
  if (!rawContent) return '';

  let htmlToSanitize = rawContent;
  if (
    !rawContent.includes('<div') &&
    !rawContent.includes('<style') &&
    !rawContent.includes('<p>')
  ) {
    htmlToSanitize = marked.parse(rawContent) as string;
  }

  return DOMPurify.sanitize(htmlToSanitize, SANITIZE_CFG);
}

const SANITIZE_CFG = {
  ADD_TAGS: [
    'style',
    'div',
    'span',
    'p',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'img',
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
    'ul',
    'ol',
    'li',
    'blockquote',
    'code',
    'pre',
    'hr',
    'br',
    'a',
    'b',
    'i',
    'strong',
    'em',
    'details',
    'summary',
    'font',
    'center',
  ],
  ADD_ATTR: [
    'style',
    'class',
    'id',
    'src',
    'alt',
    'title',
    'width',
    'height',
    'border',
    'href',
    'target',
    'rel',
    'align',
    'valign',
    'color',
    'bgcolor',
  ],
  FORCE_BODY: false,
}