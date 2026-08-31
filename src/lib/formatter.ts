import { marked } from 'marked';
import DOMPurify from 'dompurify';

// Configure marked options
marked.setOptions({
  gfm: true,
  breaks: true,
});

/**
 * Replaces macro tags like {{char}}, {{user}}, {char}, {user}, <USER>, <BOT>
 * with the actual character and user names.
 */
export function interpolateMacros(
    text: string,
    charName = 'Character',
    userName = 'User',
): string {
  if (!text) return '';
  let res = text;

  // Double curly macros
  res = res.replace(/\{\{char\}\}/gi, charName);
  res = res.replace(/\{\{user\}\}/gi, userName);

  // Single curly macros: {char}, {user}
  res = res.replace(/\{char\}/gi, charName);
  res = res.replace(/\{user\}/gi, userName);

  // XML / tag style macros: <BOT>, <bot>, <CHAR>, <char>, <USER>, <user>
  res = res.replace(/<(bot|char)>/gi, charName);
  res = res.replace(/<user>/gi, userName);

  return res;
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
): string {
  if (!rawText) return '';

  // 1. Interpolate macros first
  let text = interpolateMacros(rawText, charName, userName);

  // 2. Pre-process markdown images
  // Markdown parsers (like marked) treat raw HTML blocks (<center>...</center>, <div>...</div>)
  // as raw HTML and do not parse nested markdown images like `![alt](url)`.
  // Converting them to <img> tags beforehand ensures they are properly rendered.
  text = text.replace(
      /!\[([^\]]*)\]\(([^)]+)\)/g,
      '<img src="$2" alt="$1" loading="lazy" />',
  );

  // 3. Parse Markdown
  let html = marked.parse(text) as string;

  // Post-process in case any escaped markdown image syntax remains
  html = html.replace(
      /!\[([^\]]*)\]\(([^)]+)\)/g,
      '<img src="$2" alt="$1" loading="lazy" />',
  );

  // 4. Enhance dialogue quotes into high-contrast speech spans
  // In marked output, text quotes are escaped as &quot;...&quot; or curly quotes
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

  // 5. Convert <em> tags (from *action*) to rp-action without literal asterisks
  html = html.replace(
      /<em>(.*?)<\/em>/gs,
      '<span class="rp-action">$1</span>',
  );

  // 6. Sanitize final HTML to prevent XSS while allowing rich elements and images
  return DOMPurify.sanitize(html, {
    ADD_TAGS: [
      'span',
      'img',
      'table',
      'thead',
      'tbody',
      'tr',
      'th',
      'td',
      'details',
      'summary',
      'del',
      'code',
      'pre',
      'blockquote',
      'p',
      'br',
      'hr',
      'ul',
      'ol',
      'li',
      'a',
      'h1',
      'h2',
      'h3',
      'h4',
      'h5',
      'h6',
      'div',
      'strong',
      'em',
      'b',
      'i',
      'u',
      's',
      'strike',
      'sub',
      'sup',
      'center',
      'font',
      'figure',
      'figcaption',
      'video',
      'audio',
      'source',
    ],
    ADD_ATTR: [
      'class',
      'style',
      'src',
      'alt',
      'title',
      'width',
      'height',
      'border',
      'loading',
      'target',
      'rel',
      'href',
      'color',
      'size',
      'face',
      'align',
      'valign',
      'bgcolor',
      'controls',
      'autoplay',
      'loop',
      'muted',
      'poster',
    ],
  });
}

/**
 * Formats Creator Notes with full HTML/CSS support.
 * Allows safe styles, keyframes, grids, and tables while blocking scripts.
 */
export function formatCreatorNotes(rawContent: string): string {
  if (!rawContent) return '';

  // If content is pure markdown without HTML tags, parse it as markdown
  let htmlToSanitize = rawContent;
  if (
      !rawContent.includes('<div') &&
      !rawContent.includes('<style') &&
      !rawContent.includes('<p>')
  ) {
    htmlToSanitize = marked.parse(rawContent) as string;
  }

  return DOMPurify.sanitize(htmlToSanitize, {
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
  });
}
