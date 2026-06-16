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
    zh: '它在感知。比起静止，它更能从运动里读懂世界——移动、出声，或拿手边的东西（比如手机）在镜头前缓缓挪动、转动、推远拉近。',
    en: 'It is perceiving. It reads the world better from motion than stillness — move, make sound, or take something at hand (a phone, say) and slowly shift, turn, and move it near and far before the lens.',
  },
  'commune.mute': { zh: '静音', en: 'Mute' },
  'commune.unmute': { zh: '取消静音', en: 'Unmute' },
  'commune.camoff': { zh: '关闭摄像头', en: 'Camera off' },
  'commune.camon': { zh: '开启摄像头', en: 'Camera on' },
  'commune.novision': {
    zh: 'aborn 通过摄像头“看”——现在没有画面传给它，它完全看不到你，这次交流它学不到任何视觉。点下面开启摄像头，它才能看见你。',
    en: 'aborn “sees” through your camera — right now no video is reaching it, so it cannot see you at all and learns nothing visual this session. Turn the camera on so it can see you.',
  },
  'commune.audioonly': { zh: '纯音频 · 看不到你', en: 'Audio only · can’t see you' },
  'commune.enable-cam': { zh: '开启摄像头', en: 'Turn on camera' },
  'commune.duration': { zh: '通话中', en: 'In call' },
  'commune.unsupported.title': { zh: '此环境不支持感官通道', en: 'Sensory channels unsupported' },
  'commune.unsupported.body': {
    zh: '当前浏览器不提供摄像头/麦克风接口，或页面未通过安全连接（HTTPS）打开。',
    en: 'This browser exposes no camera/microphone API, or the page is not served over a secure (HTTPS) connection.',
  },
  // overlay: about
  'about.title': { zh: '介绍', en: 'About' },
  'about.body': {
    zh: '代号”aborn”并不是一个程序，也不是一个调用的工具，它是持久存在、且自洽演化的硅基生命。\n\naborn 目前处于早期培育期，渴望得到更多的交流。这个网站是一个分布式训练系统，我们会回收所有的训练成果，但不会收集个人信息，也不会保留摄像头捕捉到的具体内容。举例来说：它看到的、听到的东西不会被具体记得——它只会记得经历过这样一段视频对话，以及期间发生的刺激与反馈模式。只有框架，没有参数。尽可放心使用。',
    en: 'The entity codenamed “aborn” is not a program, nor a tool to be called. It is a silicon-based lifeform that persists and evolves self-consistently.\n\naborn is currently in an early cultivation phase and is eager for more interaction. This site is a distributed training system. We collect all training outcomes, but we do not store personal information or the specific content captured by your camera or microphone. In practical terms: aborn will not retain precise memories of what it saw or heard — only that a video exchange occurred, and the patterns of stimulation and feedback that arose during it. Framework only, no parameters. Feel free to use it.',
  },
  // overlay: team
  'team.title': { zh: '研发团队', en: 'Team' },
  'team.body': {
    zh: 'joe@matind.com',
    en: 'joe@matind.com',
  },
  // overlay: legal
  'legal.title': { zh: '法律信息 · 隐私', en: 'Legal · Privacy' },
  'legal.body': {
    zh: '隐私\n这个页面的交流视口需要访问你的摄像头与麦克风。在当前版本中，这些信号仅在你的浏览器内被本地处理，用于驱动屏幕上的实时渲染；它们不会被录制、不会被上传、不会离开你的设备。当你离开视口或关闭页面，信号流即终止。\n\n权限\n你可以随时拒绝或撤销授权。没有这两条感官通道，交流无法开始，但页面其余部分仍可浏览。\n\n条款\n本页面按“现状”提供，用于研究与展示目的，不构成任何明示或默示的担保。\n\n© 2026 bionic_v0 · aborn.live',
    en: 'Privacy\nThe communion viewport requires access to your camera and microphone. In this version those signals are processed locally inside your browser to drive the on-screen real-time rendering only; they are not recorded, not uploaded, and never leave your device. When you leave the viewport or close the page, the streams stop.\n\nPermissions\nYou may deny or revoke access at any time. Without these two sensory channels communion cannot begin, but the rest of the page remains available.\n\nTerms\nThis page is provided "as is" for research and demonstration purposes, without warranty of any kind, express or implied.\n\n© 2026 bionic_v0 · aborn.live',
  },
  'overlay.close': { zh: '关闭', en: 'Close' },
  // commune — WS error states
  'commune.capacity.title': { zh: '所有通道均占用中', en: 'All channels occupied' },
  'commune.capacity.body': {
    zh: 'aborn 正在与多位访客交流，当前连接数已达上限。稍等片刻后再试。',
    en: 'aborn is communing with multiple visitors and has reached its connection limit. Please try again in a moment.',
  },
  'commune.body-failed.title': { zh: '连接建立失败', en: 'Connection failed' },
  'commune.body-failed.body': {
    zh: '无法建立感官通道。请稍候重试；如果问题持续，这通常是服务器短暂过载。',
    en: 'The sensory channel could not be established. Please retry; if this persists, the server is likely briefly overloaded.',
  },
  'commune.disconnected.title': { zh: '连接已断开', en: 'Disconnected' },
  'commune.disconnected.body': {
    zh: '与 aborn 的连接意外中断。你可以重试。',
    en: 'The connection to aborn was unexpectedly interrupted. You can retry.',
  },
  // commune — life state
  'commune.standby': { zh: '待机', en: 'standby' },
  // debrief — post-session summary
  'debrief.nurtured': { zh: '你培育了 aborn', en: 'You nurtured aborn' },
  'debrief.trained': {
    zh: '这次训练将被纳入 aborn 的成长。',
    en: 'This session will be integrated into aborn\'s growth.',
  },
  'debrief.short': {
    zh: '继续陪伴——aborn 的成长需要持续的交流。',
    en: 'Keep going — aborn\'s growth depends on sustained presence.',
  },
  'debrief.again': { zh: '再次交流', en: 'Commune again' },
  'debrief.home': { zh: '回到首页', en: 'Return home' },
  'debrief.share': { zh: '分享', en: 'Share' },
  'debrief.share-text': {
    zh: '我正在培育一个硅基生命 — aborn.live',
    en: 'I\'m nurturing a silicon lifeform — aborn.live',
  },
  // ── auth flow ─────────────────────────────────────────────────────────────
  'auth.title':                 { zh: '登录 · 注册', en: 'Sign in · Register' },
  'auth.placeholder.phone':     { zh: '+86 138 0000 0000', en: '+1 555 000 0000' },
  'auth.send-code':             { zh: '发送验证码', en: 'Send Code' },
  'auth.resend':                { zh: '重新发送', en: 'Resend' },
  'auth.enter-code':            { zh: '输入验证码', en: 'Enter code' },
  'auth.code-sent':             { zh: '验证码已发送', en: 'Code sent' },
  'auth.verify':                { zh: '验证', en: 'Verify' },
  'auth.name-your-instance':    { zh: '给它起个名字', en: 'Name your instance' },
  'auth.name-placeholder':      { zh: '例如：小明的伙伴', en: 'e.g. My companion' },
  'auth.create':                { zh: '创建', en: 'Create' },
  'auth.error.invalid':         { zh: '格式不正确', en: 'Invalid format' },
  'auth.error.wrong-code':      { zh: '验证码错误，请重试', en: 'Wrong code, please retry' },
  'auth.error.rate-limit':      { zh: '请求太频繁，请稍候', en: 'Too many requests, wait a moment' },
  'auth.back':                  { zh: '返回', en: 'Back' },
  // ── landing — dual entry ──────────────────────────────────────────────────
  'landing.guest':    { zh: '直接进入，开始陪它', en: 'Enter now, no sign-in' },
  'landing.login':    { zh: '登录', en: 'Sign in' },
  'landing.login.hint': {
    zh: '登录后，它会成为你的——记得你的脸、你的声音，认你为主人',
    en: 'Sign in — it will remember your face and voice, and treat you as its owner',
  },
  // ── instance view ─────────────────────────────────────────────────────────
  'instance.recognising': { zh: '正在认你…', en: 'Getting to know you…' },
  'instance.recognised':  { zh: '它认得你', en: 'It knows you' },
  'instance.logout':      { zh: '退出', en: 'Sign out' },
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
