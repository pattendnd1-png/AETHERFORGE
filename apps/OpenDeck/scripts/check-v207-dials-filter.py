from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
actions = (root / 'apps/opendeck-studio/src/components/ActionLibrary.tsx').read_text()
model = (root / 'apps/opendeck-studio/src/model/actions.ts').read_text()
test = (root / 'apps/opendeck-studio/src/App.test.tsx').read_text()

checks = {
    'DIALS_MODE_REQUIRES_DIAL_SUPPORT': "mode === 'keys'\n        ? supportsControl(item.id, 'key')\n        : supportsControl(item.id, 'dial');" in actions,
    'DIALS_MODE_DOES_NOT_PROMOTE_TOUCH_ONLY': "supportsControl(item.id, 'dial') || supportsControl(item.id, 'touch')" not in actions,
    'FOLDER_HAS_NO_DIAL_SUPPORT': "id: 'editor.folder'" in model and "supportedKinds: ['key', 'touch']" in model,
    'REGRESSION_TEST_PRESENT': "filters non-dial actions out of Dials mode" in test and "name: 'Folder'" in test,
}
for key, passed in checks.items():
    print(f'OPENDECK_V207_{key}={"PASS" if passed else "FAIL"}')

bad = [key for key, passed in checks.items() if not passed]
if bad:
    print('OPENDECK_V207_DIALS_FILTER_CONTRACT=FAIL:' + ','.join(bad))
    sys.exit(1)
print('OPENDECK_V207_DIALS_FILTER_CONTRACT=PASS')
