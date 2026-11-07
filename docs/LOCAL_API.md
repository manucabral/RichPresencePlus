# Local address

Overlays and stream decks can read what Discord shows. Turn it on in Settings. It is
off by default, and off it holds no port.

| Address | Answers |
| --- | --- |
| `http://127.0.0.1:28752/v1/now` | JSON with what Discord shows now |
| `http://127.0.0.1:28752/overlay` | A page for a browser source in OBS, with a clear background |

`/v1/now` gives `showing`, `presence`, `details`, `state`, `large_image`, `small_image`,
`start` and `end`. With nothing on display `showing` is `false` and the rest are `null`.
It reports the activity after your privacy rules, the same one Discord received.

## Limits

- It listens on `127.0.0.1` only, so it cannot be reached from another computer.
- It only reads. There is no way to change anything through it.
- The `Host` header must be `127.0.0.1:28752` or `localhost:28752`. Any other value gets
  a 403, which stops a website that points its own name at this computer.
- A request with an `Origin` from anywhere else gets a 403, and no answer carries a CORS
  header, so a website you visit cannot read it. A browser source that needs it should
  load `/overlay`, or a program should ask it directly.
- It serves eight requests at once, and a connection lasts at most five seconds.
