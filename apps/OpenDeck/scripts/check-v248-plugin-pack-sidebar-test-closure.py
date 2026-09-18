#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
sidebar=(root/'apps/opendeck-studio/src/components/AppSidebar.tsx').read_text()
test=(root/'apps/opendeck-studio/src/components/AppSidebar.test.tsx').read_text()
checks={
  "sidebar_accessible_label": "Plugins & Packs" in sidebar and "aria-label={item.label}" in sidebar,
  "test_uses_new_label": "'Plugins & Packs'" in test,
  "test_does_not_expect_legacy_label": "['Touch Strip', 'Profiles', 'Plugins', 'Settings']" not in test,
}
failed=[k for k,v in checks.items() if not v]
for k,v in checks.items(): print(f"OPENDECK_V248_{k.upper()}={'PASS' if v else 'FAIL'}")
if failed:
  print('OPENDECK_V248_PLUGIN_PACK_SIDEBAR_TEST_CLOSURE=FAIL:'+','.join(failed))
  raise SystemExit(1)
print('OPENDECK_V248_PLUGIN_PACK_SIDEBAR_TEST_CLOSURE=PASS')
