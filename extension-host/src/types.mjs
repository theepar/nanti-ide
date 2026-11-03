// Plain value types of the VS Code API. They hold data and do no work for the IDE, so they are real, not stubs.

// TypeScript enums are objects that map both ways (name -> number and number -> name).
function enumOf(members) {
  const out = {};
  for (const [name, value] of Object.entries(members)) {
    out[name] = value;
    out[value] = name;
  }
  return Object.freeze(out);
}

export const DiagnosticSeverity = enumOf({ Error: 0, Warning: 1, Information: 2, Hint: 3 });
export const DiagnosticTag = enumOf({ Unnecessary: 1, Deprecated: 2 });
export const EndOfLine = enumOf({ LF: 1, CRLF: 2 });
export const FileType = enumOf({ Unknown: 0, File: 1, Directory: 2, SymbolicLink: 64 });
export const ViewColumn = enumOf({ Active: -1, Beside: -2, One: 1, Two: 2, Three: 3, Four: 4, Five: 5, Six: 6, Seven: 7, Eight: 8, Nine: 9 });
export const TreeItemCollapsibleState = enumOf({ None: 0, Collapsed: 1, Expanded: 2 });
export const TextEditorRevealType = enumOf({ Default: 0, InCenter: 1, InCenterIfOutsideViewport: 2, AtTop: 3 });
export const SymbolKind = enumOf({
  File: 0, Module: 1, Namespace: 2, Package: 3, Class: 4, Method: 5, Property: 6, Field: 7, Constructor: 8,
  Enum: 9, Interface: 10, Function: 11, Variable: 12, Constant: 13, String: 14, Number: 15, Boolean: 16,
  Array: 17, Object: 18, Key: 19, Null: 20, EnumMember: 21, Struct: 22, Event: 23, Operator: 24, TypeParameter: 25,
});

export class Position {
  constructor(line, character) {
    if (line < 0) throw new Error("line must be non-negative");
    if (character < 0) throw new Error("character must be non-negative");
    this.line = line;
    this.character = character;
  }
  isBefore(other) {
    return this.compareTo(other) < 0;
  }
  isBeforeOrEqual(other) {
    return this.compareTo(other) <= 0;
  }
  isAfter(other) {
    return this.compareTo(other) > 0;
  }
  isAfterOrEqual(other) {
    return this.compareTo(other) >= 0;
  }
  isEqual(other) {
    return this.compareTo(other) === 0;
  }
  compareTo(other) {
    return this.line - other.line || this.character - other.character;
  }
  translate(lineDelta = 0, characterDelta = 0) {
    if (typeof lineDelta === "object") {
      return this.translate(lineDelta.lineDelta ?? 0, lineDelta.characterDelta ?? 0);
    }
    return new Position(this.line + lineDelta, this.character + characterDelta);
  }
  with(line = this.line, character = this.character) {
    if (typeof line === "object") return this.with(line.line ?? this.line, line.character ?? this.character);
    return new Position(line, character);
  }
}

export class Range {
  constructor(a, b, c, d) {
    let start;
    let end;
    if (typeof a === "number") {
      start = new Position(a, b);
      end = new Position(c, d);
    } else {
      start = a;
      end = b;
    }
    if (start.isAfter(end)) [start, end] = [end, start];
    this.start = start;
    this.end = end;
  }
  get isEmpty() {
    return this.start.isEqual(this.end);
  }
  get isSingleLine() {
    return this.start.line === this.end.line;
  }
  contains(x) {
    if (x instanceof Range) return this.contains(x.start) && this.contains(x.end);
    return x.isAfterOrEqual(this.start) && x.isBeforeOrEqual(this.end);
  }
  isEqual(other) {
    return this.start.isEqual(other.start) && this.end.isEqual(other.end);
  }
  intersection(other) {
    const start = this.start.isAfter(other.start) ? this.start : other.start;
    const end = this.end.isBefore(other.end) ? this.end : other.end;
    return start.isAfter(end) ? undefined : new Range(start, end);
  }
  union(other) {
    const start = this.start.isBefore(other.start) ? this.start : other.start;
    const end = this.end.isAfter(other.end) ? this.end : other.end;
    return new Range(start, end);
  }
  with(start = this.start, end = this.end) {
    if (start && typeof start === "object" && !(start instanceof Position)) {
      return this.with(start.start ?? this.start, start.end ?? this.end);
    }
    return new Range(start, end);
  }
}

export class Selection extends Range {
  constructor(a, b, c, d) {
    const anchor = typeof a === "number" ? new Position(a, b) : a;
    const active = typeof a === "number" ? new Position(c, d) : b;
    super(anchor, active);
    this.anchor = anchor;
    this.active = active;
  }
  get isReversed() {
    return this.anchor.isAfter(this.active);
  }
}

export class Location {
  constructor(uri, rangeOrPosition) {
    this.uri = uri;
    if (rangeOrPosition instanceof Range) this.range = rangeOrPosition;
    else if (rangeOrPosition instanceof Position) this.range = new Range(rangeOrPosition, rangeOrPosition);
  }
}

export class TextEdit {
  constructor(range, newText) {
    this.range = range;
    this.newText = newText;
  }
  static replace(range, newText) {
    return new TextEdit(range, newText);
  }
  static insert(position, newText) {
    return new TextEdit(new Range(position, position), newText);
  }
  static delete(range) {
    return new TextEdit(range, "");
  }
  static setEndOfLine(eol) {
    const edit = new TextEdit(new Range(new Position(0, 0), new Position(0, 0)), "");
    edit.newEol = eol;
    return edit;
  }
}

export class DiagnosticRelatedInformation {
  constructor(location, message) {
    this.location = location;
    this.message = message;
  }
}

export class Diagnostic {
  constructor(range, message, severity = DiagnosticSeverity.Error) {
    this.range = range;
    this.message = message;
    this.severity = severity;
  }
}

export class ThemeColor {
  constructor(id) {
    this.id = id;
  }
}

export class ThemeIcon {
  constructor(id, color) {
    this.id = id;
    this.color = color;
  }
}
ThemeIcon.File = new ThemeIcon("file");
ThemeIcon.Folder = new ThemeIcon("folder");

export class MarkdownString {
  constructor(value = "", supportThemeIcons = false) {
    this.value = value;
    this.supportThemeIcons = supportThemeIcons;
    this.isTrusted = undefined;
    this.supportHtml = undefined;
  }
  appendText(value) {
    this.value += String(value).replace(/[\\`*_{}[\]()#+\-.!|<>~]/g, "\\$&").replace(/\r?\n/g, "\\\n");
    return this;
  }
  appendMarkdown(value) {
    this.value += value;
    return this;
  }
  appendCodeblock(code, language = "") {
    this.value += `\n\`\`\`${language}\n${code}\n\`\`\`\n`;
    return this;
  }
}

export class TreeItem {
  constructor(labelOrUri, collapsibleState = TreeItemCollapsibleState.None) {
    if (typeof labelOrUri === "string" || (labelOrUri && typeof labelOrUri === "object" && "label" in labelOrUri)) {
      this.label = labelOrUri;
    } else {
      this.resourceUri = labelOrUri;
    }
    this.collapsibleState = collapsibleState;
  }
}

export class FileSystemError extends Error {
  constructor(messageOrUri, code = "Unknown") {
    super(typeof messageOrUri === "string" ? messageOrUri : String(messageOrUri));
    this.name = `${code} (FileSystemError)`;
    this.code = code;
  }
  static FileNotFound(messageOrUri) {
    return new FileSystemError(messageOrUri, "FileNotFound");
  }
  static FileExists(messageOrUri) {
    return new FileSystemError(messageOrUri, "FileExists");
  }
  static FileNotADirectory(messageOrUri) {
    return new FileSystemError(messageOrUri, "FileNotADirectory");
  }
  static FileIsADirectory(messageOrUri) {
    return new FileSystemError(messageOrUri, "FileIsADirectory");
  }
  static NoPermissions(messageOrUri) {
    return new FileSystemError(messageOrUri, "NoPermissions");
  }
  static Unavailable(messageOrUri) {
    return new FileSystemError(messageOrUri, "Unavailable");
  }
}

export class CancellationError extends Error {
  constructor() {
    super("Canceled");
    this.name = "Canceled";
  }
}

export function makeCancellation(EventEmitter) {
  return class CancellationTokenSource {
    #emitter = new EventEmitter();
    #cancelled = false;
    token = {
      isCancellationRequested: false,
      onCancellationRequested: this.#emitter.event,
    };
    cancel() {
      if (this.#cancelled) return;
      this.#cancelled = true;
      this.token.isCancellationRequested = true;
      this.#emitter.fire(undefined);
    }
    dispose() {
      this.#emitter.dispose();
    }
  };
}

const DEFAULT_WORD = /(-?\d*\.\d\w*)|([^`~!@#%^&*()\-=+[{\]}\\|;:'",.<>/?\s]+)/g;

function lineStartsOf(text) {
  const starts = [0];
  const re = /\r\n|\r|\n/g;
  let m;
  while ((m = re.exec(text))) starts.push(m.index + m[0].length);
  return starts;
}

/** A TextDocument over a text snapshot; `_update` swaps the snapshot the way an edit does, keeping the object identity. */
export function makeDocument({ uri, fileName, languageId, version = 1, text = "", isDirty = false }) {
  let content = text;
  let starts = lineStartsOf(content);

  const lineText = (line) => {
    const end = line + 1 < starts.length ? starts[line + 1] : content.length;
    return content.slice(starts[line], end).replace(/(\r\n|\r|\n)$/, "");
  };

  const doc = {
    uri,
    fileName: fileName ?? uri.fsPath,
    languageId,
    version,
    isDirty,
    isUntitled: false,
    isClosed: false,
    get eol() {
      return content.includes("\r\n") ? EndOfLine.CRLF : EndOfLine.LF;
    },
    get lineCount() {
      return starts.length;
    },
    validatePosition(position) {
      const line = Math.min(Math.max(position.line, 0), starts.length - 1);
      const character = Math.min(Math.max(position.character, 0), lineText(line).length);
      return new Position(line, character);
    },
    validateRange(range) {
      return new Range(doc.validatePosition(range.start), doc.validatePosition(range.end));
    },
    offsetAt(position) {
      const p = doc.validatePosition(position);
      return starts[p.line] + p.character;
    },
    positionAt(offset) {
      const clamped = Math.min(Math.max(offset, 0), content.length);
      let lo = 0;
      let hi = starts.length - 1;
      while (lo < hi) {
        const mid = (lo + hi + 1) >> 1;
        if (starts[mid] <= clamped) lo = mid;
        else hi = mid - 1;
      }
      return new Position(lo, Math.min(clamped - starts[lo], lineText(lo).length));
    },
    getText(range) {
      if (range === undefined) return content;
      const r = doc.validateRange(range);
      return content.slice(doc.offsetAt(r.start), doc.offsetAt(r.end));
    },
    lineAt(lineOrPosition) {
      const line = typeof lineOrPosition === "number" ? lineOrPosition : lineOrPosition.line;
      if (!Number.isInteger(line) || line < 0 || line >= starts.length) throw new Error("Illegal value for `line`");
      const text = lineText(line);
      const last = line === starts.length - 1;
      return {
        lineNumber: line,
        text,
        range: new Range(line, 0, line, text.length),
        rangeIncludingLineBreak: last ? new Range(line, 0, line, text.length) : new Range(line, 0, line + 1, 0),
        firstNonWhitespaceCharacterIndex: text.search(/\S|$/),
        isEmptyOrWhitespace: text.trim() === "",
      };
    },
    getWordRangeAtPosition(position, regex = DEFAULT_WORD) {
      const p = doc.validatePosition(position);
      const text = lineText(p.line);
      const re = new RegExp(regex.source, regex.flags.includes("g") ? regex.flags : `${regex.flags}g`);
      let m;
      while ((m = re.exec(text))) {
        if (m[0] === "") {
          re.lastIndex++;
          continue;
        }
        if (m.index <= p.character && p.character <= m.index + m[0].length) {
          return new Range(p.line, m.index, p.line, m.index + m[0].length);
        }
        if (m.index > p.character) break;
      }
      return undefined;
    },
    async save() {
      return false;
    },
    _update(nextText, nextVersion, nextDirty) {
      content = nextText;
      starts = lineStartsOf(content);
      doc.version = nextVersion;
      doc.isDirty = nextDirty;
    },
  };
  return doc;
}
