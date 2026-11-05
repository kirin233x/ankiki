// 音频播放队列：按顺序播放一面卡片中的所有发音
import { mediaUrl } from "./api";

let current: HTMLAudioElement | null = null;
let queue: string[] = [];
let queueDir = "";
let token = 0;

function playNext(my: number, onFirst?: (ok: boolean) => void) {
  if (my !== token) return;
  const next = queue.shift();
  if (!next) {
    current = null;
    onFirst?.(false);
    return;
  }
  const audio = new Audio(mediaUrl(queueDir, next));
  current = audio;
  let done = false;
  const fail = () => {
    if (done) return;
    done = true;
    console.warn("无法播放音频", next);
    playNext(my, onFirst);
  };
  audio.onended = () => {
    if (done) return;
    done = true;
    playNext(my);
  };
  audio.onerror = fail;
  audio
    .play()
    .then(() => {
      const cb = onFirst;
      onFirst = undefined;
      cb?.(true);
    })
    .catch(fail);
}

export function stopAudio() {
  token++;
  queue = [];
  if (current) {
    current.onended = null;
    current.onerror = null;
    current.pause();
    current = null;
  }
  if ("speechSynthesis" in window) window.speechSynthesis.cancel();
}

/** 播放一组音频，返回是否至少有一个成功开始播放 */
export function playSounds(mediaDir: string, files: string[]): Promise<boolean> {
  stopAudio();
  if (!files.length) return Promise.resolve(false);
  queueDir = mediaDir;
  queue = [...files];
  const my = token;
  return new Promise((resolve) => playNext(my, resolve));
}

/** 根据文字猜测朗读语言 */
export function guessLang(text: string): string {
  if (/[\u3040-\u30ff]/.test(text)) return "ja-JP";
  if (/[\uac00-\ud7af]/.test(text)) return "ko-KR";
  if (/[\u4e00-\u9fff]/.test(text)) return "zh-CN";
  if (/[äöüß]/i.test(text)) return "de-DE";
  if (/[àâçéèêëîïôûùÿœ]/i.test(text)) return "fr-FR";
  return "en-US";
}

/** 使用系统语音朗读，返回是否可用 */
export function speak(text: string, lang?: string): boolean {
  if (!("speechSynthesis" in window) || !text.trim()) return false;
  window.speechSynthesis.cancel();
  const u = new SpeechSynthesisUtterance(text);
  u.lang = lang ? lang.replace("_", "-") : guessLang(text);
  u.rate = 0.9;
  window.speechSynthesis.speak(u);
  return true;
}
