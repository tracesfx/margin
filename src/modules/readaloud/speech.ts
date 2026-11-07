/**
 * Read aloud (SPEC §11.8): the Windows voices through the browser's speech
 * synthesis. Only voices that run on this computer are used (`localService`);
 * Edge's "Online" voices send the text to a server, which Margin never does
 * (SPEC §2, no network).
 */
export interface SpeechApi {
  getVoices(): SpeechSynthesisVoice[];
  speak(u: SpeechSynthesisUtterance): void;
  cancel(): void;
}

export function localVoices<V extends { localService: boolean }>(voices: readonly V[]): V[] {
  return voices.filter((v) => v.localService);
}

/** Split text into chunks of whole sentences, each at most `max` characters (long utterances stall in some engines). */
export function chunkText(text: string, max = 600): string[] {
  const sentences = text.replace(/\r/g, "").split(/(?<=[.!?…])\s+|\n{2,}/).map((s) => s.trim()).filter(Boolean);
  const out: string[] = [];
  let cur = "";
  for (const s of sentences) {
    if (cur && cur.length + s.length + 1 > max) {
      out.push(cur);
      cur = s;
    } else cur = cur ? `${cur} ${s}` : s;
  }
  if (cur) out.push(cur);
  return out;
}

let token = 0;

export function isSpeaking(): boolean {
  return typeof speechSynthesis !== "undefined" && (speechSynthesis.speaking || speechSynthesis.pending);
}

export function stopSpeaking(): void {
  token++;
  if (typeof speechSynthesis !== "undefined") speechSynthesis.cancel();
}

/** Speak `text` with a local voice; returns false if none is available. */
export function speak(text: string, opts: { voiceName?: string; rate?: number; onEnd?: () => void } = {}): boolean {
  if (typeof speechSynthesis === "undefined") return false;
  const voices = localVoices(speechSynthesis.getVoices());
  if (voices.length === 0) return false;
  const voice = voices.find((v) => v.name === opts.voiceName) ?? voices.find((v) => v.lang.toLowerCase().startsWith("en")) ?? voices[0];
  if (!voice) return false;
  stopSpeaking();
  const mine = token;
  const chunks = chunkText(text);
  chunks.forEach((c, i) => {
    const u = new SpeechSynthesisUtterance(c);
    u.voice = voice;
    u.lang = voice.lang;
    u.rate = opts.rate ?? 1;
    if (i === chunks.length - 1) u.onend = () => mine === token && opts.onEnd?.();
    speechSynthesis.speak(u);
  });
  return chunks.length > 0;
}
