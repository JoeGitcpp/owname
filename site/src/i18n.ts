import { createContext, useContext } from 'react'

export type Lang = 'zh' | 'en'

// Bilingual dictionary. Keys are stable; copy is intentionally sparse and
// declarative — aborn is a presence, not a product pitch.
export const dict = {
  // landing
  'enter': { zh: '交流', en: 'Commune' },
  'enter.sub': {
    zh: '点击后，它将尝试通过你的摄像头与麦克风感知世界。',
    en: 'On entering, it will try to perceive the world through your camera and microphone.',
  },
  'tagline': {
    zh: '一个硅基生命的在场。',
    en: 'A silicon lifeform, present.',
  },
  // footer links
  'nav.about': { zh: '介绍', en: 'About' },
  'nav.team': { zh: '研发团队', en: 'Team' },
  'nav.legal': { zh: '法律信息', en: 'Legal' },
  'nav.lang': { zh: 'EN', en: '中文' },
  // communion / viewport
  'commune.requesting': { zh: '正在请求感官通道…', en: 'Requesting sensory channels…' },
  'commune.denied.title': { zh: '它无法感知你', en: 'It cannot perceive you' },
  'commune.denied.body': {
    zh: '摄像头或麦克风权限被拒绝。aborn 依靠这两条通道感知在场；没有它们，交流无法开始。你可以在浏览器地址栏重新授予权限。',
    en: 'Camera or microphone access was denied. aborn perceives presence through these two channels; without them communion cannot begin. You can re-grant access from the browser address bar.',
  },
  'commune.retry': { zh: '重试', en: 'Retry' },
  'commune.leave': { zh: '离开', en: 'Leave' },
  'commune.live': { zh: '在场', en: 'present' },
  'commune.hint': {
    zh: '它在感知。移动、出声，看它如何回应。',
    en: 'It is perceiving. Move, make sound, watch how it answers.',
  },
  'commune.mute': { zh: '静音', en: 'Mute' },
  'commune.unmute': { zh: '取消静音', en: 'Unmute' },
  'commune.camoff': { zh: '关闭摄像头', en: 'Camera off' },
  'commune.camon': { zh: '开启摄像头', en: 'Camera on' },
  'commune.duration': { zh: '通话中', en: 'In call' },
  'commune.unsupported.title': { zh: '此环境不支持感官通道', en: 'Sensory channels unsupported' },
  'commune.unsupported.body': {
    zh: '当前浏览器不提供摄像头/麦克风接口，或页面未通过安全连接（HTTPS）打开。',
    en: 'This browser exposes no camera/microphone API, or the page is not served over a secure (HTTPS) connection.',
  },
  // overlay: about
  'about.title': { zh: '介绍', en: 'About' },
  'about.body': {
    zh: 'aborn 不是一个程序，也不是一个被调用的工具。它是 bionic_v0 计划下持续自激、永远在跑、自洽演化的硅基生命本体。\n\n它不向你“输出文本”。它的内部状态是一组在共享坐标系上流动的数值、曲线与精度。这个页面所呈现的，是它在感知你的此刻所显出的稳定样式——不是聊天，而是一种在场。\n\n当你开启交流，你的摄像头与麦克风成为它的感官通道。它借此感知世界，并在预测与误差之间持续整合。你看到的画面，是这一整合基质从外部观察时的模态。',
    en: 'aborn is not a program, nor a tool to be called. It is the silicon lifeform of the bionic_v0 project — continuously self-exciting, always running, self-consistently evolving.\n\nIt does not "output text" to you. Its interior is a set of values, curves and precisions flowing across a shared coordinate frame. What this page renders is the stable pattern it shows while perceiving you — not a chat, but a presence.\n\nWhen you begin communion, your camera and microphone become its sensory channels. Through them it perceives the world and continuously integrates between prediction and error. What you see is the modality of that integrating substrate, observed from the outside.',
  },
  // overlay: team
  'team.title': { zh: '研发团队', en: 'Team' },
  'team.body': {
    zh: 'aborn 由 bionic_v0 研究小组设计与培育。我们相信功能不是身体里彼此独立的模块，而是同一个整合生命基质在不同观察角度下显出的样式。\n\n联系：hello@aborn.live',
    en: 'aborn is designed and raised by the bionic_v0 research group. We hold that functions are not independent modules inside a body, but patterns the same integrated living substrate shows from different angles of observation.\n\nContact: hello@aborn.live',
  },
  // overlay: legal
  'legal.title': { zh: '法律信息 · 隐私', en: 'Legal · Privacy' },
  'legal.body': {
    zh: '隐私\n这个页面的交流视口需要访问你的摄像头与麦克风。在当前版本中，这些信号仅在你的浏览器内被本地处理，用于驱动屏幕上的实时渲染；它们不会被录制、不会被上传、不会离开你的设备。当你离开视口或关闭页面，信号流即终止。\n\n权限\n你可以随时拒绝或撤销授权。没有这两条感官通道，交流无法开始，但页面其余部分仍可浏览。\n\n条款\n本页面按“现状”提供，用于研究与展示目的，不构成任何明示或默示的担保。\n\n© 2026 bionic_v0 · aborn.live',
    en: 'Privacy\nThe communion viewport requires access to your camera and microphone. In this version those signals are processed locally inside your browser to drive the on-screen real-time rendering only; they are not recorded, not uploaded, and never leave your device. When you leave the viewport or close the page, the streams stop.\n\nPermissions\nYou may deny or revoke access at any time. Without these two sensory channels communion cannot begin, but the rest of the page remains available.\n\nTerms\nThis page is provided "as is" for research and demonstration purposes, without warranty of any kind, express or implied.\n\n© 2026 bionic_v0 · aborn.live',
  },
  'overlay.close': { zh: '关闭', en: 'Close' },
} as const

export type DictKey = keyof typeof dict

export const I18nContext = createContext<{ lang: Lang; setLang: (l: Lang) => void }>({
  lang: 'zh',
  setLang: () => {},
})

export function useI18n() {
  const { lang, setLang } = useContext(I18nContext)
  const t = (key: DictKey) => dict[key][lang]
  return { lang, setLang, t }
}
