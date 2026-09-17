from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
app=(root/'apps/opendeck-studio/src/app/App.tsx').read_text()
prop=(root/'apps/opendeck-studio/src/components/PropertyInspector.tsx').read_text()
inspect=(root/'apps/opendeck-studio/src/inspector/ActionInspector.tsx').read_text()
test=(root/'apps/opendeck-studio/src/App.test.tsx').read_text()
checks={
  'APP_OWNS_INTERACTION': "const [selectedInteraction, setSelectedInteraction] = useState<Interaction>('press')" in app,
  'ASSIGN_USES_SELECTED_INTERACTION': 'interactionOverride?: Interaction' in app and 'onChoose={(id) => assignAction(id, state.selection, selectedInteraction)}' in app,
  'PROPERTY_INTERACTION_CONTROLLED': 'interaction: Interaction;' in prop and 'onInteraction: (value: Interaction) => void;' in prop,
  'INSPECTOR_LABEL_ACCESSIBLE': 'aria-label="Interaction"' in inspect,
  'UI_REGRESSION_TEST_PRESENT': 'assigns actions to the selected dial interaction including press-plus-rotate' in test,
}
for key, ok in checks.items(): print(f'OPENDECK_V207_ASSIGN_{key}={"PASS" if ok else "FAIL"}')
bad=[k for k,v in checks.items() if not v]
if bad:
  print('OPENDECK_V207_INTERACTION_ASSIGNMENT_CONTRACT=FAIL:'+','.join(bad)); sys.exit(1)
print('OPENDECK_V207_INTERACTION_ASSIGNMENT_CONTRACT=PASS')
