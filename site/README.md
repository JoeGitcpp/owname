# aborn.live

The presence page for **aborn** — the silicon lifeform of the `bionic_v0` project.
Not a marketing site: a single immersive page whose viewport perceives the
visitor through camera + microphone and renders the living modality of the
integrating substrate (drive / err / precision / re-entry), rather than a chat.

## Structure

- **Landing** — one large `交流 / Commune` button over an ambient idle field,
  with small footer links (`介绍 / About`, `研发团队 / Team`, `法律信息 / Legal`)
  and a `中文 / EN` toggle.
- **Commune viewport** — requests camera + mic (`getUserMedia`), then drives the
  modality renderer from those live signals. Camera contours steer a particle
  flow field; audio sculpts the breathing presence core.

Bilingual (zh / en), state in `src/i18n.ts`. No router — the app is two stages
in `App.tsx`.

## Privacy

Camera and microphone signals are processed **only in the browser** to drive the
on-screen rendering. They are not recorded, uploaded, or persisted. See the
in-page Legal overlay (`legal.body` in `src/i18n.ts`).

## Architecture

```
src/
  App.tsx            two-stage shell (landing ⇄ commune) + language provider
  i18n.ts            bilingual dictionary + hook
  lib/
    media.ts         getUserMedia acquisition + teardown
    perception.ts    camera/mic → tier-0 sense bundle (Senses)
    modality.ts      the renderer: particle field + presence core + readouts
    idle.ts          quiescent senses for the ambient landing field
  components/
    Field.tsx        mounts a canvas and runs Modality against a sense source
    Overlay.tsx      About / Team / Legal reading overlay
  views/
    Landing.tsx      the big button + footer links
    Commune.tsx      media lifecycle (requesting / live / denied / unsupported)
```

The renderer is decoupled from the sense source via `Field`'s `read()` prop, so
a future version can swap `Perception` for a WebSocket feed of the real
`bionic_v0` runtime snapshot without touching the visuals.

## Develop

```bash
npm install
npm run dev        # http://localhost:5173
npm run build      # → dist/  (static, deploy anywhere)
npm run preview    # serve the production build locally
```

## Deploy

Builds to a fully static `dist/`. Any static host works; camera/mic require
HTTPS (all of these serve HTTPS by default):

- **Cloudflare Pages** — build `npm run build`, output `dist`. `_headers` is honored.
- **Vercel** — framework "Vite", output `dist`.
- **Netlify** — build `npm run build`, publish `dist`. `_headers` is honored.

Point the `aborn.live` domain at whichever host you choose.
