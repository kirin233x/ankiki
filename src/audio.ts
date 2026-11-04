// 简单的音频播放队列：自动按顺序播放一面卡片中的所有发音
import { mediaUrl } from "./api";

let current: HTMLAudioElement | null = null;
let queue: string[] = [];
let queueDir = "";

function playNext() {
  const next = queue.shift();
  if (!next) {
    current = null;
    return;
  }
  const audio = new Audio(mediaUrl(queueDir, next));
  current = audio;
  audio.onended = playNext;
  audio.onerror = playNext;
  audio.play().catch(playNext);
}

export function stopAudio() {
  queue = [];
  if (current) {
    current.onended = null;
    current.onerror = null;
    current.pause();
    current = null;
  }
  if ("speechSynthesis" in window) window.speechSynthesis.cancel();
}

export function playSounds(mediaDir: string, files: string[]) {
  stopAudio();
  if (!files.length) return;
  queueDir = mediaDir;
  queue = [...files];
  playNext();
}

export function speak(text: string, lang?: string) {
  if (!("speechSynthesis" in window) || !text) return;
  window.speechSynthesis.cancel();
  const u = new SpeechSynthesisUtterance(text);
  if (lang) u.lang = lang.replace("_", "-");
  window.speechSynthesis.speak(u);
}
