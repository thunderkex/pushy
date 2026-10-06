# Provider Status Report

Live verification conducted on 2026-09-30 as part of Phase 0.

| Provider | Enabled by Default | Phase 0 Status | Response Format | Sample URL / Response Body | Download Verification | Notes |
|---|---|---|---|---|---|---|
| `catbox` | Yes (Default) | PASS | Plain URL | `https://files.catbox.moe/w7nzjn.txt` | PASS (`hi`) | Working as expected. Max limit ~200 MB. |
| `litterbox` | Yes | PASS | Plain URL | `https://litter.catbox.moe/iy3xra.txt` | PASS (`hi`) | Working as expected. Retention times: 1h, 12h, 24h, 72h. |
| `tempsh` | Yes | PASS | Plain URL | `https://temp.sh/MCYBQ/t.txt` | PASS (Page 200 OK) | Working as expected. 3 days retention, 4 GB limit. |
| `gofile` | Yes | PASS | JSON | `https://gofile.io/d/5HZoZFsH` | PASS (Page 200 OK) | Returns `data.downloadPage` & `data.guestToken`. Guest token saved/reused. |
| `tmpfiles` | Yes | PASS | JSON | `https://tmpfiles.org/wdAUwIERYyz6/testfile.txt` | PASS (Page/File 200 OK) | Short names like `t.txt` rejected by API (`Invalid file name`); use valid/padded filename. Direct URL rewritten with `/dl/`. |
| `zerox` (0x0.st) | No | FAIL (Disabled upstream) | Plain text error | `uploads disabled because it’s been almost nothing but AI botnet spam for the past few months. will be back with a few changes at some point. no ETA.` | N/A | Off by default. Detect "uploads disabled" message. |
| `fileio` | No | FAIL (Broken anonymous) | HTML redirect | HTTP 301 -> Limewire web page | N/A | Off by default. Anonymous uploads broken upstream; requires API key. |
| `transfersh` | No | FAIL (Host unreachable) | Connection failed | `curl: (7) Failed to connect to transfer.sh:443` | N/A | Off by default. Public instance offline; requires custom base_url. |

## Verified Active Providers for Fallback Chain

1. `catbox` (Primary default)
2. `tempsh`
3. `gofile`
4. `tmpfiles`

`litterbox` is active when expiry `-e` is specified or selected with `-p litterbox`.
