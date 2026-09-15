from pathlib import Path
import re
p = Path(__file__).resolve().parents[1] / 'apps/opendeck-studio/src/app/App.tsx'
s = p.read_text()
m = re.search(r'async function beginTwitch\(\) \{(?P<body>.*?)\n  \}', s, re.S)
if not m:
    raise SystemExit('TWITCH_POLL_UI_CONTRACT=FAIL:BEGIN_TWITCH_NOT_FOUND')
body = m.group('body')
checks = {
    'poll_call': 'bridge.twitchPollAuth' in body,
    'interval': '.interval' in body or 'interval' in body,
    'expiry': 'expires_in' in body,
}
missing = [k for k,v in checks.items() if not v]
if missing:
    raise SystemExit('TWITCH_POLL_UI_CONTRACT=FAIL:' + ','.join(missing))
print('TWITCH_POLL_UI_CONTRACT=PASS')
