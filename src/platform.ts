// 运行平台判断：iPhone / iPad 上使用手机布局。
// 在电脑浏览器里调试时，可以在地址后加 ?mobile 强制使用手机布局。
export const isMobile =
  typeof navigator !== "undefined" &&
  (/iPhone|iPad|iPod|Android/i.test(navigator.userAgent) ||
    (typeof location !== "undefined" && new URLSearchParams(location.search).has("mobile")));

/** 轻触反馈（支持的设备上震动一下） */
export function haptic(ms = 8) {
  try {
    navigator.vibrate?.(ms);
  } catch {
    /* 忽略 */
  }
}
