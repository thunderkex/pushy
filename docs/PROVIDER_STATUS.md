# Provider Status Report

Automated health check conducted on: **2026-10-20 03:12:53 UTC**

| Provider | Status | HTTP | Sample URL / Response | Download | Notes |
|---|---|---|---|---|---|
| `catbox` | **FAIL** | 412 | `Invalid uploader` | N/A | Default host. ~200 MB max. |
| `litterbox` | **FAIL** | 412 | `No file!` | N/A | Temporary files (1h, 12h, 24h, 72h). 1 GB max. |
| `tempsh` | **PASS** | 200 | `https://temp.sh/DrYjW/t_probe.txt` | PASS | 3 days retention, 4 GB limit. |
| `gofile` | **PASS** | 200 | `https://gofile.io/d/q4rZuso6` | PASS | Unlimited size, saved guestToken reused. |
| `tmpfiles` | **FAIL** | ERR | `Connection failed / timed out` | N/A | 100 MB max, ~1 hour retention. URL rewritten to /dl/. |
| `zerox` | **FAIL** | 503 | `uploads disabled because it’s been almost nothing but AI botnet spam for the past few months. will be back with a few ` | N/A | Off by default. Upstream disabled uploads. |
| `fileio` | **FAIL** | 301 | `<html><head><title>301 Moved Permanently</title></head><body><center><h1>301 Moved Permanently</h1></center><hr><center>` | N/A | Off by default. Anonymous uploads broken; requires API key. |
| `transfersh` | **FAIL** | ERR | `Connection failed / timed out` | N/A | Off by default. Public host unreliable; requires custom base_url. |

---

*This report is automatically updated by the [Provider Health Check workflow](.github/workflows/provider-health.yml).*
