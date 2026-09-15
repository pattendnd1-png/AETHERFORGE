#!/usr/bin/env bash
set -uo pipefail

DL="$HOME/Downloads"
TASK12_ANCHOR="c65cbde0fde36ac28f0f3e6694a038903ba28d46"
COMMIT_MSG='feat(integration): open managed paths in Aether Terminal'
TMP="$DL/.AetherFiles-v0.1.0-TASK13-tmp"
PATCH="$TMP/AetherFiles-v0.1.0-TASK13-AETHER-TERMINAL-UX-PATCH.zip"
PATCH_ROOT="$TMP/patch"
LOGDIR="$DL/AetherFiles-v0.1.0-TASK13-logs"
CURRENT_LOG=""
ROOT=""
START_HEAD=""
MUTATED=0
COMMITTED=0

mkdir -p "$DL" "$TMP" "$LOGDIR"
say(){ printf '%s\n' "$1"; }

resolve_root(){
  local candidate
  for candidate in \
    "$DL/AetherFiles-v0.1.0" \
    "$DL/ForgeClean/Projects/AetherFiles/Active"; do
    if [[ -f "$candidate/Cargo.toml" && -d "$candidate/.git" ]]; then
      ROOT="$(cd "$candidate" && pwd -P)"
      return 0
    fi
  done
  return 1
}

print_tail(){
  if [[ -n "$CURRENT_LOG" && -f "$CURRENT_LOG" ]]; then
    printf '\n=== FIRST FAILURE DIAGNOSTIC (tail) ===\n'
    tail -n 240 "$CURRENT_LOG" || true
  fi
}

rollback(){
  if [[ "$MUTATED" -eq 1 && "$COMMITTED" -eq 0 && -n "$START_HEAD" && -d "$ROOT/.git" ]]; then
    cd "$ROOT" >/dev/null 2>&1 || true
    git reset --hard "$START_HEAD" >/dev/null 2>&1 || true
    git clean -fd -- \
      crates/aetherfiles-ui/src/terminal_action.rs \
      crates/aetherfiles-ui/tests/terminal_action.rs \
      tests/test_aether_terminal_ui_contract.sh \
      >/dev/null 2>&1 || true
  fi
}

fail(){
  local reason="$1"
  rollback
  printf '\n=== AETHERFILES TASK 13 RESULT ===\n'
  say "AETHERFILES_VERSION=0.1.0"
  say "AETHERFILES_TASK=TASK13_AETHER_TERMINAL_UX"
  say "AETHERFILES_V0_1_0_TASK13=FAIL:$reason"
  say "AETHERFILES_HEAD_BEFORE=$START_HEAD"
  print_tail
  say "AETHERFILES_LOG_DIR=$LOGDIR"
  say "AETHERFILES_HANDOFF=PASTE_TERMINAL_RESULT"
  rm -rf "$TMP" >/dev/null 2>&1 || true
  exit 1
}

run_gate(){
  local name="$1"; shift
  CURRENT_LOG="$LOGDIR/${name}.log"
  local rc=0
  "$@" >"$CURRENT_LOG" 2>&1 || rc=$?
  [[ $rc -eq 0 ]] || fail "$name"
  say "AETHERFILES_TASK13_${name}=PASS"
}

say "AETHERFILES_VERSION=0.1.0"
say "AETHERFILES_TASK=TASK13_AETHER_TERMINAL_UX"
say "AETHERFILES_TERMINAL=/usr/bin/aether-terminal"
say "AETHERFILES_TERMINAL_FALLBACK=NONE"
say "AETHERFILES_FORGECLEAN_TERMINAL_RESOLUTION=ACTIVE_PATH_WINS"
say "AETHERFILES_TASK13_UX=CONTEXT_ACTION,TOOLBAR_ACTION,SCROLLABLE_SIDEBAR,ENTER_SEARCH,EMPTY_STATE,STATUS_VISIBILITY"
say "AETHERFILES_ARTIFACT_POLICY=DOWNLOADS_ONLY"
say "AETHERFILES_RUSTC=$(rustc --version 2>/dev/null || echo MISSING)"
say "AETHERFILES_CARGO=$(cargo --version 2>/dev/null || echo MISSING)"

resolve_root || fail "PROJECT_ROOT_NOT_FOUND"
say "AETHERFILES_ROOT=$ROOT"
for cmd in cargo git python3 unzip sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || fail "${cmd^^}_NOT_FOUND"
done

python3 - "$PATCH" <<'PY2'
from pathlib import Path
import base64, sys
payload = r"""UEsDBBQAAAAIAIm8L12rEsRwHgEAAKUBAAAMAAAATUFOSUZFU1QudHh0RdDBbsIwDAbge5/C91Fa
KKugnKppnKcJJK5u4tCMNKkcl423X0CDXXJw/uj/4pakJ95ZRxEu5XwxL2GP8QyLKst2hDIxNdDe
Q7AnHqxHB2Ekn6cJwQvsAp/ozRF6aJXYC+UjSg9MMbhJbPAp04XJa9JwOMKIMWatV33gBlT9qjpN
pdFU1aiWa1Oaiup6s8KyWm/KqsPlWq/q7NHcQDFFLjrrC7ybcnmavLtm/5gGPm+CC31w+CIl2z8d
3HXf1sctBGOc9QQGnYvQoTqDBFATM3kBbTm9C3zNDsfmMc1NcDqt4lmL6vbHGYzJYoUGUMEL/cjz
IioOzmHnCKLV1CHP4D1FEj3kkZBVPwMaRrnmUVCouJ1TTAs0yTakzpj9AlBLAwQUAAAACABpvC9d
f8cmOLkZAAAiiwAALAAAAGdyZWVuL2NyYXRlcy9hZXRoZXJmaWxlcy11aS9zcmMvYXBwX3N0YXRl
LnJz7T1rb9y2lt/9K5QpMNAsJtPHbu8CcuOLNElvu2hvunFu90MQCBqJ9uhGI031sOPaA+yv2R+2
v2TP4UskRVKSY3e7j6CoNRJ5SB6eFw8PD7uGBGmdtCSKLvKCxFc5uY6i24YUF+vgO3jzC7x4nrZ5
VR5PTzqldFVfkrQgSSnqfIdvXuAbVn4dvK4vkzL/jdTfk6RodzqA6kDgAcrFzQ5ak42+Fu/P8bVe
pyFJne7ibVLL4uf0lWiR/fqpyohRMc+IVov9tg2sTbaNLPcWflgLkXqfl0kRJ/QjlD8J4N9b/vrH
pCvT3Rvya0eadh0U9GeckHZH6ljUXQeHuvo7Sdu4TPYkBoTGh6TdrYOaNFVxRWTBOL3O1idGB3Zk
D39e1sllVf6lSJpG/1wd1OG+pT+1gfDO5GVGPkK576v2B3wk9TqA5zdV1TbrgL46b6uarIO8vITB
xDW5zJu2vtHBsIkRaHhRlS0pWzYZrw/YaiPm5kVSpqR4W30g2nQFSRO8JGzEAD9P+0+i2L92pL5Z
0wY4HaSsnfhX9kXrUpLHlKD1nj3P3yTlB5K9IU1XtHyASOasiZ/r6gooA3BwftO0ZP+cgnqevyhy
aAdmBioDxH1SQg+TojAbxRYb6FZNkB/gx6sSULUOfqzSRBBoQeg0nLcwUZTA+BMymkK4DGBeASAN
ZS/zGgBU9c2rstsjp1S1yjMIi0NQODSl/dd4VAzpZ0aD52VyaHYVvJBUCzg6wMQh/gFq14giHHzT
ZlGEBAtgf6Z0i///trtQvzc3ZRpF+0OTSlp8Q1KSXyGO39Y38OPqVV1XtVqpzZG0b192NUfaD2XT
AsqhzEl7cyAA4gJ4ZMcmMXgWsIdvwh7Nv5D0G4n/sxUOoQYCPjtlAHo0sKGZkPQBm7UZrfxLtTWr
Yavs48u8ORTJDXt/ZgIQYsICguNQqXHy2TugSMBY+JJsu8t18KKoSpiUF9XhBnFet3lSvPp1Hbz6
dfX+5NBtAYl1l7bB88OBkkPASF8hhx0Vx5EpnzlvUcqPgX+SqyQvkm1BomBbVQXQ+slJvj8UJuTP
3u07kAwwf+/pb+wDsGbTBhclNguCDD5f5+0uVjpRXVwUeUnCVfD0DNnigkPDf8bPad2PotcM5Fqr
aB3PRVI0SrnjCfv/hOGgTNnW1XUDUo+UCC0Ll0jZdByIJ6XjMBFkOmhzhApcY6hKE1hkM6g6uVEL
dtzDoW1ZaqitIUzSxla4+64NmBAwSGs1oRHgEfl8amnRgr6+PQfJrEYRCa2yB9Gk4IGX5CJBzoUa
A04rg4x9dRI3tZ9G+KJvkDN0L7V6wcHA5lkUdH/6J0bQ3CQJBvYY/VrTepGQN6qQpwKHcrlV5IB0
IaBzAouECzS9r4yV6gehGHp+a8r8cCDQC2YcfCPbpkNmf4TuDXWdvaIdVMUcLYfCvoF54G1TI45q
AaFgz3rkoGmVbAFjDVACew29rnPCq/SKQ1gbXGdHpvqmn9OurtEKSamalmNStPaZQD1VW3H9URYS
yvAbTaWd8QqKicxVEe+hru8F+K6kmjNoqw9oN4jf0Rv2wEoVXEfGJMthAhjm11xDFDDtaDaBHaZ/
4gbXHuyTSLHatI+MsngPrWpQK21BgqFYz/QaBn4H5iQvLa1mWwsDzSvboBpfH/SuQmxqxJscDjEW
hfeC6demdhWgJG2rtsSZo7i1t3YzRXQZjH5N6h2qArDDbaVBM/gVrH3AzlUCxaRxZZb7e7WlfVcX
YbZCIx0eIPhAygwwizb7RV7vE8ZPHIApqsSkHJKUAF2lFQAnQsZJK8TG+CgYQP6W5DoEPNRA4b1R
mLbQ5SW57PIooqLqY2uTz8qaKopgwoubEGquTk9kiYK0gttAP5js9m2XIx9FEXQi3gMK8rjd1STJ
uEgX/zbMfIhhHWF+2SKIQfGPB5A94UIZNyzsoG3Rl4XZRyRg6CA1qkl5FUVXSR1XTbj4/vVPrxYG
+H1yCDmtg3aqq73xvSuv6+QQg9oiYDuFd3eBVjpcfL5YmR1w0R90ShAgqw2LqrQJv16datVRf8ME
QGmLQchE/BVJn7wTMp6inM37JkWVFa76dRUslkkL9kOzeq8biKpK+EL/pKoFBnxlGJcu1RBF0gTQ
a5ja4q+VabCqamL41aUTrN2rVckvMatrAIaujElqE4BVJTgwYdUQUUQJ1VpWVRg+mHZEGFphWEBT
BJb6msS3to/8Y9DKUP47Z9qiEoa9sCuCYTmvsIfOV9eGuKA8m+5ICpZT3HTb0MWLlmoGq6utuMeo
AXWWsqmXKRgUGmeIGbtiscy3qUwUXj/2cgdmeENZIuZ8iMJfSJOVVs5YhPRyRFlBLYVsUqQX/bpB
+fWOPvUV3ztBgiJp+wUNA42/bOBFqUlN6GNVlkxUWxrKsm8kv6AS+hw4JGQsuEJNg23pEg468QEX
+brw5t/Yn1BB61FXIEJWCdgKjlebMrnKL+ln+MFbxSc2V7oqYS0BGMVg5DyvF7yu6g8g8WR50VMb
UF5W6aR4tJcP24/roP6ImGIusXSXlCVDgL4O1XGIClBB9KlettcXolz90SzDZAt8B5YCFnkSLn6s
EmSb4Pa4WPfdlmpg0COuSjbARtdlmKBjL9gDKxkTS00j4dOy+CoVXh/KHcKLkdDAbK/NtemxgEiu
k7wdvkbj5o5r87vABL8Win61slaNSV1DdfRT3gX0z6atQF6j4tCEgsBAjGbhx00Doilk2DAKoUyp
maMVeO+Q5Ei5ChesTA8Hla4DLlXZSrIjIzCDNAQbokA3pqwmbVeXp6owlM9AK+kuwOr1DcrOqwEn
v/4Qwn+hBZXQizOjMJccU1k5ePYsWEruGoKSxMnbhGHzp1N3WWmybVCz1OHKV3bANrfHIG/JvgGm
UVveFKQcUELvWRwAHvA36iu99tFE8ysgQkp8LszqXaZFTx+oeWxbdddH0av9ob3xdWRELE1p42Xe
gFYvYbpINmnMCylxhBXNBUaQKaAWyL3VdUky2+TPR9DR4NV9crMlMVOsignj0rFLn5I1vZLSSNzk
TdwgVnWRBcsyWsVmL25IASsGHHXwzQCwZr5JkCOSwtARtkYBd7qVOl0lujyzEgdWwmIaiIpUVN+9
8fYQGo3JQ3MXDXrNaAuEF1VAFkIFBma7cJSQGRz2QiiCkO+8RRHz9rhhcXjCdysqhAwxTDzAd/5z
7apfoR+BFkbWkyKuK5mngWTKQPlyCTHB2gxuafXoz0dY9tub6AWWaGOoO4c1j8NX8wEd76GTOdHc
Qy/7mHxMRVt5+3GVtUomDokq17nWid3Y91sG+3HUL8Ra27AyK5fAHaBB8Dbv7ATdGHtHZGtgKNQf
GAF8Q3L1wKp1six8SCX7+2DE1KXcMAQ7sVIVpzAKR927xv6esaA2XJy9Fer5SkJzVWKqKs3Bpi4K
5epqoNsUL9vQLB0zW4UN5PVZWFC6TdIPM6wR/3qcYWyAwg1thC/KNXtL9SAIsCsFWwYdzsbsVOwq
GFa9kt6C3hXEpOnw0jrw0XVSZ48+N6Kd/5+eWdPTHWbODF9N8InRpYq6DHZJI/OLXCDrrxHdBY0h
NL+AoYSv4213odnB6iQfkppZp6K7G/ZmYERQBKpyWYhgvpVDO8ChUfR4cArGflYgEIxP1AJE+K6K
Grk4imtmALGqRqdVOFH0LQglVHX6UKiowv6uPVW/YzwzrC2YaQzA3w7DukBRY9X+UqlseDachUJV
hF5QPNZAQnEzgw/KOWlxay3E6F+67eM1HkxZhOaCrKnAmGoXCMJJto2VbGTc7icRjYQSRfj3CrU/
jZhdIevQJ7GUxp4wL9BUt1fw5BkH4fFsKcV5aY+7ypS/k13mDvk8V05/ip9tVBzb7dmjc8L+Sq7t
M/Fg+wraEHD+7ZbxoYMx6VvUQ7/2cJfa1ZBGESbpPQ2+dFSbq5vnzvtkHT2HQGYRh5sUXhRVozKu
ibWz4MtguZzHz32hmqDXiIM/ncL4Z8OJmyoCJk34kXkOTBGlAJrW3NNnVugnv5vw+d9CgFJ5xKvg
zkKaMSOyUXXHj7HYNJ52ouVTlJ4GCMSoueqdb30YEHunYsg60EMUrjfFkcCK8JImdAea5EkmG6L0
Q02fgikdElpESE+vy+ImzLN+UD2Zsae44iXWXmhvq8vLgjggtfKjH8jrA0iJ+8+cDdwPJQtAE6GU
YXqtdLGCEnFemmedsNCUBh6RNlw9U5dyTCb1MZ9TN2iUUKN+X8a2bOp3qhgS5VEQfkYsyJsgKTBY
8AZkNx4QuwSh07h3rDxbMmjn8BNm0KD1rFm45EO+766MMvA5wQdIRxh8YCIBhnx7/M9//4+FnAst
IuExt3hENJ2MSTUORp2h/a0ul9nnVY9ai1K9/3bR3C2jN+zM4M+iK7JTR38bg60kDiEU+GBOdfFr
7QNClRvzxGOdYQiWZ4tH+KZ9VQ3L37Y9gv+wsrkZNJhukEcwc7aTlpIn1nLUm6RB9W8N9eAzAcCs
hz9hJgBmhP8TRGrbnJIbjPajouGSN7NyhLaEqzts4vcKX7n/VpnENBvo9H0yVcQ+7vYY4nFStIEU
Z6YYQ1VDMibNUJTB3NvE2CNFeXzy9tKYUH+c0A2HQpwVv+E2oN3Wg81EHJxkH7cDWF3KEXlNDwH3
4ah2W8AWuCrwzY0fhZBt1M4ikD6StGP+R49d5HbBuus/EGbMLUsWyjvZTlKDELgZrhhJoOHRkri3
gcSomwtXPGLIpjFLYFGPu/lMza6c2B8IgwGZ05CxJigrwGV3OFQ1BlZsbwKgdrkxoAyRtbwYaj/e
syLZEvO8yNiQjVMZeWYJPsDA7M2WXIKF3L8N9UYfIHaHTb4zcIcHZvJ5FtPxPyy4hz3anYGCopz6
3BPFOlGf/w7xM+6TqJrnK3NRcaRP9LAYmxOjT48RrwP0OHMzzxPAwyXbI5on0IKyGIBfm9qFfSgt
grUcCtjBn82m6dKUgChF+Hnmc9wLh4saAMXaDJdYW6igWh4z1pXR0Is4JYTQuQjRDrx4Ixw9MHyn
X6Z4Q3Vum4H2iyQvOM7XnK/d2yCTA36NVdP6AYOPPML8cWxERU/Kc3APZSF6SHl4tKbHohBoS/tx
ey7MeMykPG1/ohwEMAUO+zj0QQowBpr6pbsItsrCptvvk/pm1irGSmEaxgX4gAfBh83qFJ+bD3ia
P1vYV/W8LxvUvkDX4ievNKjysE74oV9Dc96492opyknzRBrR5vRG0fmuuv4pKZNLkv0og2KCzQa0
jo/rRyMnWN82+GPlkpdO54dnlUrPJfOMDHRdqrbkXKCObbkOMYz+l9nsLNxV1wkzlS+qrpzExnof
XlRFRvNVZb0Da9ZSHgEEDYXAsdQ7gep0l18RL6YehFThadIQ0BqhXu3kApSaMGCRaK1k4bK1qB9+
QN28Ex63Yd90RBPduH2Dm43dDTiF+YEXlTF+Al/JecxgZHnJdkQekr/EtJl0o7TnpZ0Hj4boKeoX
UucX+X2Zwq4ZnjN2CK44bBx2EN4eVy494OSjkfLNLvnq6z+N0Ypr6CzwmPmmBwcm4tlyir/sDlky
0c7Qu0NNH6956KAupQ+3R05i3EakOfLY8540DSikuUYP3zZ3+330LIufsLmswKEhZBjzEnqixzzn
jLUEAPZjxqPHjUdiGmQCAv9hgZlRBzJRAops+HM6QgMMn7SCfXiDdAuIywU+PcXtvoCFTuCGG3rO
aEiGSrnrUaAisROCFc9sHAiReuLmAeTJnX5wr5CYjBHFeGNdA2NDxxkNdeXJywKlVH2Vp9rYJiqe
o497DaLttvu8NYInWf8sQZx6XeTf+v8MobODP9ZCbomk4XP+UUmNu55ZOGPsRKNqXNG8or2fRw1D
aut8H66s4h+6Qiuih5vganrMxc06yVsDfqKV7uXR/jQi8iRlmEgsXiLRnbhV9ceJh7+XU1vFwv1O
oTK5PiBbm8Y0ZPvt8WSCqLavGPqUEmYCNSU3g5//bVkpPDaqldxRG2GcVsCT+DYY9jF+MFvz68db
mOIP6L+mrv27O4dUVL38TmWjJBnGvLZ1QzBaiIanL1mCYVeCYol0t1ORkXaZxaCpyvCOgs/uPKqv
H/Qw03E4Wov6qpDF1tOKsv5MLGzL7uxM4eOEwqhmvLAfp8cRlNPgCC6JpqCbF500hA3wdhXnQBLh
aloFpTd3tjSaPffeTgKI/5g/hwGlnpz15KoyCyevzX9PA3CcOOa0Kgq6efap8zgrimVWNMuMDSXX
XpRbHvstXexfBhKvqJKsEaoa045t/l7lZbh4Kb4txuXq8wZFocM0biuavJwFhJMMtUCW4xr2PnLX
vZ9q3Vdlpd0kTcPvqLPpmZJxPopQ/JJ9BQJvNYEA/nzqbQClIaJJJLcHiYUbBlfAvUs5A2smNVd+
UAyHNQPGs+aL1IPQ8ZFNG2rnVnWwq4DC0ArKS9a5Deuin/XBwhMV0b7M8tq5FtCqsV5OlWtQmDYR
LkVjE7ndvXF9D5b2zKh9KTP+BdDHcc2vMUAkohkyikXj+oNw2bBrEZY6vHE8fdrAjyde0jzwCwwE
Jw2vNlAIdYTMWRgDWnmWuxDALk7KqsRbEEIPnMGFCeHyIO9YWKb8DgJhX335xRdjJtQoff0BVb5V
0wt0zlOjUUQvGYjPzh5doZ5M5yw2LX8UDXy0hXyYDoWxqA65upsXzTE31pQT31iuNI9XQ2p435Eu
kbpj40rNj/s3M7xFtrRo3EXHRhQ2q8VasJYzOZrFXcCCsvFp4hL0IRKn/W5optdO3APPvlxu93E+
P2beN79f5IHiQv7wMyacevdLPzdnmgYO1F11HetEwfLRroMuj1gIC3Oj/i33eFAFPTmdmF3Ow0In
ubF8OwqvMCYrSIQniDtCYYEC5gxpGl5vY9lXnLelIBsqYZLqpHhaJOVll1wS7q9VNhLa5KbhKylY
ObW7vOErqs1iAinY92MWY+GMTrcum67ztAZN9rwmSRTBsqVlttcG5zvs8nVw1+V3w0tzxDIMVhlL
m7wdogom9rKuOrCiEKCMPbJHGfbDdXkyAnapGXcuBEf/lg80DhZIBRaIHpUyYa1vOsB5i1Sl88b9
1qCkaFHzdOYK4zgdPfJ2E24PTcXKUvH0xLu89HVSjkeq6XN6NVlwG23+8Sj186bBt5OxiuJyJdW0
mYuMTMWxZWMbOsZyq9Ocgqf3W9494f1isTAZLO5IkXnE2HgHf2KQ+J67HTzz1qwD6Pe9Oi5bFfML
XN5oB0BG9i/d1vAgBJoqCO14yj0CoOXpXPdZFnnebWYgtLj5gcNCMxOV8PBqCKkeje+s9/+Wl1l1
zZa7uGlNYQ3Okpi3YOA6Kzk0ea/79e8wPflviesrlcUp7iBZhLGVuuyKCsxpGBxzsFERTp+aqqtT
EmRJm2wcQTaeQyLUmW4hKNklNcQEw6Qv67y9CTCkCrf497AQC5Ku3VXwOsE9us3CDg1L/AakA9rJ
gQXOqlB227UtEKCYnwVmWckxgtvLpz1l4LrldIYsNlqlBDS1UUFtc9oc8KWRfUuMZPrxsKEpeL9T
YDJFCB/XfXtgmp9bjAZJ626v3oLA7thwJ2OckBzEDFwCIcSPsPcXIuCxfbMR6kxPGnUbuL/EjVzU
CV7Zg/cH0YvcbFcKQWss3GtETq6DmIHjNjaH/R3+GSSi1PK5m1vLqsfC+s08RWstZMlKO1YOT8Po
dEKLjSS0Nm5HYkh5Wx2+rYDN9j8nJSnojbThQrkl9Sm8WDy4cJ0me3pzSr1MaeHQtlJAsoG9ydPd
W5hu7khF/Cg0v1ptrknywR6ruTo9se4AYVo0Ee5ozZzjTD2lWma9Sha3/8qlwVJtYa3cEKIl7fGG
reuZ+uxCxeOkdnSU3rXcd1NmN1qbSJjUN0wGN7tnk6mmTxswiFqZFsEyLZplQmTLtCiXKREvA93o
htDsExAWQnViBg9cUxqntheeLgCV7XC/LUZhySGoh2BZgNVMoFJzW4u4tfm0ZDVzTAsUKuSQ0FtO
wvuIkol3TIwIGKfAUS+EE4qRk40vHsqWN9uehNp+543tRlXD9zZMG23AMK5ukj56/bW5AMC0U1z5
FOSi1bUPGBUpinxdxfAglvg6z6AXX331xeYL85Y8IBf29ct//nrwtVdg1KE+U3/hUOXJZ5/Px7EJ
06GPZAeW1ofwHbvgmK2M3jsq+DxHGq+w/F1cUI+snv3RI7o15i87JDCRhmVyvZ1yr7VzdDoVuQs7
dsQcyM3LktRTFbY1Vt6m4kQCuDEt55QCduNsS3/oHMJw/t9oonGHiOJXd0vV6ZK394XRZGr9cR09
wRqXtQ/ZoLz0rlGbtNzn+AiNu0PsNfcbet7Wfp3pFdvWqg8yEJlFs9+xsezWrKYcJoiYY/9zdmzB
ztS+4182YA4wE3Q0/lNogN2HaFjExkFxn5bnK1I0GZKCc7cMzfTzp1008e0CfT1hbvaoSwotTn7l
2iW0HjxyS7XjiY9yTmZuDdpOAggFLFNRWxdifcrqyYsxmYzSpz9denOpGoC+IlJy2QvJfvtgaDO3
9uiiESU+Za3Wp+icpcpcfMlwYNn57HKvZ/xkCEDzjPedok6jz96lF5dhi4lb3p8AOgN8FJTUQc+a
7oAhVv/AIX/2Dgu8F26kvMzbHBZruBrHo0xxxnI9M0SYfnaW8FdPHG072Lr4vN0fFtYc0sp1pU0D
5mNMfn0SAlyVjt31jtZBVOwGF9URVBJc1mV5g0K4ibd1dd2QejAgtj/0TLk5F/NQInAwqWH1oUDk
jYSDETwJufJB+uENxewi7Uy/n7UfsFNdrYOR+2lw0v8LUEsDBBQAAAAIAGm8L112zDAy9gkAAHMl
AAAsAAAAZ3JlZW4vY3JhdGVzL2FldGhlcmZpbGVzLXVpL3NyYy9maWxlX3ZpZXcucnPtGu1u47jx
v5+C0QKpdNXqkmtue1CaBZJsbhvksru4JIcCaSDQEm2z0YeXkpJ1YwP3EIfrz/bZ7gn6CJ0hKYmS
ZTvZ3esHUAOJJXI4nO8ZDl3mjISCFsz3R5kYszBmNA3uOLv3/YcBgc9hWPAsPbyjPKZDHvNi5pJv
EfQYQdWsS4Y0GrMAULgkoSkdsyjIC0CLQ8GUFhN3sNgflM1uBRMJT2kcUInBGAjvo3qVWkJZMWFi
xGOWB2EmYPXDt/BykhYCiYHH00h9n/EUnr7LQqrIumAxk/gvkBiX/ACMnWcR07QYLIcxZ2nh++eK
egmvgPIi8n0kBrZ9h5wQ/H9UjhbGfMETIOvqzemfgpN3b4//uD8YPLuOmOB3zH7FhuXYJcdxljJc
LApO45P3Ljl579wMpuWQsLRMJP1InxIpUdJXDLxN45mt+HRcOX6Zjccxa4+9nbLUrng3xk7TQynA
Sy1hW9OvQRpl2l29AsRiICkcpSSfZPe2XFFyn2wnZUHYGB79K64QgfwEZznMXdfquVFTeaUHmOzo
RM6jwQUJKMavVaQmQOlgGLAqL4QamWQgabItNTFwyPOXwCNi+0Nbfi+1APmIbGnCPA6obMej6cye
49hsDsZahBMwK7WPzZRJqTfH0TgUz94dA82FaKEAxgSL7HnJ5waMhqMRGP+Uhsze+8bbcfa78xNG
I56ObSBNbeQBeQkQxvOAJdNiZjsdpPixLic8B4uNwaoIPElIqwW2ICwGg+xZ+yZTnMK2RPpRZ+Ey
keDsLLaXMCmNf8/DySX7AO6Ssvsn8iHpQV3lhKaRZign0xgEFhEwU0bueRwTOp0yKggtiwxIR7nH
M89aQreSZ7nP8YSmY0bA+jWJJBMEHV7IsRwewglMsTgiRSZNHCYaOdMx5Wnfrs7SkHfP6K2tfar6
GHI1ZSxYUYqUvIGAoAbBy/BL6qhxBoOryit8/xUrIBTnZG6MHWcJ2FtBDl5KFoJIwdgldyuvdBsf
rOzblb5kUNwgPA2zNK/RcXx7CrIFhg0dMmpifqXQ8ckRImYFQYpUIiIHWi2DxtwvQpHF8aFg1PeH
WTEBE5fBECXSCQFqwWvBI+UblpG7nmtJPB/DtNW2IA8yAOS2uExA0l935oAzPoVwU4iSdadA7xhL
rndfeDsugX83XYgVlBq+DvgzwGG9oQmzOqGgAyIFvwmG/3UTyOVsugkEDJGDY0YrwFgaBQI4c7Sm
zA+kdWlRM8JT0gn+OtA/Of6bH7SYFGQFtiIXePjiFVmAikrHQZzl+czuobtarGwawt1BY94eOBmY
B6h/WyHl0RoMguVTcEokAWUmkdBhzAIVt6sNXEmnR/NAsJHt9AlLJ8kKoRdlJeKBeii8BZtzyPa2
5vIWiitycFAXWhCLuIBtMhB0v6DwU7vVBbil3XZB35c1i0If68rFC7FQAmJXsK8jvklzQ+xj6ajF
ruvPOrqh3DdSMFitWfQP2KS/CLYxNFX71PziQ6zmV+2ItgH5NkigUrS3K741Jlft6qrAqvhZgadO
7HW9bsu1jhretArgwWGCHBxcKw0fXcM+nE0oEEgbqVzlPHLPRIeDjauMwLCst76iZ1F5hZKcmbpU
4vsfS1zVlM4mMG7bWJWqExy49j2P0NTIl2T36x0oUL1RnIEhOITmpESFOl5CP9i7zsokWFXCH5sI
pVw3pkH9vTLj7WHG2/2IlFcJDuFQbjuS6/3/SB759FSQz5JhFsN6iIifEqitf/79bz9a60LumrW/
/PyPFUv3/92xElHL0AGoVfDYslcT/qDEt/hz+gB/1w+LG8tdCb0u269e1R9re8EflfFXblSdPcGY
I/CP38mS8JuvwEFc7Y9HZVFg2kWPVBHfqwywrhn+XyV8rirhc2XyXkQqeP32gOzur9KWBMHTeFLG
BZ+CvrJRHVXXCGF9Fu3n7Sm5tSWDgToQKzlA9tMdhkowdZadwVzT9VMZFoXlE7Nl5xq76YTdn0Mx
wT609vZaZHVSB3ogRp265O+NSBgV8CkYliNTdLja7G8Clt52p70tW6WbPcnADXoGfQ2lY9sWegpm
K9XxI1XLz3JWWv8Xm11vqYFoEu8s947AMXKmxNiZVI0Poyli9tdyNqWCFliKGDYvJS+yvzDsb6yN
wB5PYQxMHrlsC1/xiLXQHQu3rls0dZuevn+e3bHLDN/u2Du99bK3yJaw/F9FguUUoAn3q4d+yIW7
gaDjLAbzzgSk4J5tN68Hb1KrlwjE/hdwuYmPCBCAwpVXYZm5htX+6QzsKKAjWfGOaJyzJ4rgv4wF
bMU8nYMsvmNHJY8jLPH7TOrzmMsPTPDR7FDJ5eMktnGPC8gtOvBWVw6/HkMnH8K4jD7e+nlqrjdX
3BiRBsv9kKYRj7AuhSBaRY02XzKeGNdhEFTqVZ45ocu9/aXVLMVzGGaBNp6Dnss2YF5BL6Np94Cw
8tOIbf3dV/U1pKrj9VL50sEM6eUxRF6leTmdZgLqx6PZK8qSXnOoUy1UVBMIsiLAhGtb37P3JaS2
nFAyKiFDMEOLJFLYwIKKDIoXAitYmgMFnbNjU/FtVQJ+PAVXaX0+lnYgLwEMInLV93zcdUu1yaBz
EBp06rOKTKidH1WkbkjVxkVerWWnv3u6OkMvOrkZKzjdEVkuhuty7QrqYjjp67tY2brovc6UvhNF
HDOrMlueTsvCnsuvOZFfnu71iBxKsiTBW6J5z1Qhqn4V2miFtJFaVzz6yhTO0oNe7XXhjavXeo0W
Rd9pv1Wetpo7UhzDDEy3vpJ8xH2ZviPCRNO6I5LNZ8aiGAVY4cmgBrpnIqQ5qxQqiRo0jRUomZq3
pdqpNWXgasbrnoTaHHvKgALih5ZJt7FnykNKYPs36EQ8BF8SjST0aQqQzZKYp7eGBKzv4N3qV5a6
KTPq5La3NCUzPuFFlrxy7Bzt+46oElRe/q0EfitLawSUTwbcomUlZrcU//mkfLHnSkH5NTopmgup
i0YojztDawuxfvnxJ8vrVr7aWrDRVpCz0yOoul7sgcXs7ny15+3sG5PnzSTAkS96QF43IOdLIGiQ
w1nBsLpGNrGNCLA63AE3avLlAaIxyK96M9aD7+0uyGt+ZLka9ksEbbmpieZ8HZrzFprzNWjO1qE5
a6E566DpW4asL8iR5fTZQLt7veQaLf13HLda2oxAQAzA8FJ7Xs3NSfXkRaWQxRiYXRoyu/lFiuNl
t+itDWI6tecV+JxUT9KxWZir06yKEq11ZXov6DSAIyvKwobQ3LVAGRKeXYejMZwU88K5GQB5BB+r
Wkr+bqacQoT0DS/Zr+eWf/FTuYI2q2fXiO5GNdJT+ZsGiSIPqGDBpISsEQhGI0ywrbBK85yJImDv
t1q3GWjQbidsOC6xdr0daQtGllyB4fdu70HdVdKpvXIx+BdQSwMEFAAAAAgAabwvXcj/0GpiAgAA
SAUAACYAAABncmVlbi9jcmF0ZXMvYWV0aGVyZmlsZXMtdWkvc3JjL2xpYi5yc21UXW/TMBR9z6+w
9jClUqgKD2h4MKllHRtsLWonXhCy3OSmMTh2sJ12g/W/448kTSP8kPRen+Nzfe5NS5khWlVEG2rg
MiptmDMOZMdg34RSbSHlQEUvKStQ1DApiC6A5yGpgaq0IBuqmphl0AWGbnTzC1TJBOWEpu6EJllA
2egbWXlWVNUbVGs41ofx3ylYpLqxJeppVSXIPtZu63DZwQcFW1KE7Jp6temOMk43jDPznKAZzbaQ
oBvH+OgYAZSgpdpSwf6AugXKTZGgNfidtaCVLqRJ0MZRidVK/Ok67BPdAHQS9Uoa2GVLWraZL0xk
Vq8N125/GH+Wm2FqeOmj9/bwtQ/au4ToQWZ9/KAJnUnzx9v5ijzOVw93i+k9md0tEvTYYO9pLdJi
rpS99CC5gt81aGsL9yGhvk2kVQkeVUr+tDYRQUvvHKmos1aBlnwHHZike+tIPzqC+6b6kcH4WtGt
FJ841Xa+Ij8vXtyNsSapVBZzL1MaZs17ZTKM3XEYf7XPWZ03w5YLpGph6zNsB7EdOWUwaiAj9OoK
Qa6o01yBrrl5H4+uUPCNg0GBRmTllDT60KEXfmPZ5APBLTedlXQasK0Zxt+aeFYznoFtZAY5tTrx
qKO4Nd4zUxAmhDVY2xmNv79+czEZTxJ0MbGvH/9DWytPGO8mnvA2EJKOMR5fB82huPXdv9s79Xzq
yGe9j/PseOapL8f8TD5hLGAfl3IH6CVVED6RVAoDT+al55Rby19xxzj9Fwi5E7BbbdMxzpUs/fzE
576pvQu363woP3Y9Ial5OsWORkd7D805o+gQ/QNQSwMEFAAAAAgAabwvXSo3cA7kAgAAYQcAAC0A
AABncmVlbi9jcmF0ZXMvYWV0aGVyZmlsZXMtdWkvc3JjL3NlYXJjaF9iYXIucnONVF1r2zAUfc+v
uG0hk8HLw/rmLIXQDwjLukG6h1FCkO3rWlT+qCWty5r89+nDju04lBoSX0lX9x6dc+SLxxgr9gfJ
DYbqyYdrXuSoX0W59eEnrSSj/PbFB/O7wYQqLr31qFQhYK4yWCGtovR7ESO8jUA/F7qezVrb4R3j
KHwbXhe5xFy6wRxlitV84Y/2oxHLSj6sZHpERS4kzJfLAB5XyJMpXK5h5uIgcMXBDZry9bBpsJ6O
jqolOXAaIidCJ3rw+QrGn4SkkkUgZFU3N09GZZSCyepMmqfTHWZXcG6jc/9ETg3KZtXxybwGrU1s
Bp3M/cj9a7IuPizYUKd5JFmR16dZoTRsk5Z4zzVcqTBjtU7XXK9akUwpTZ1Ii1eiWADjTEnAJx0G
v5gPmd4fdET04UVhta3zVrJi+ZMl+0dpMHztArqqEXGUYLKpgzmDe320qV1SbJIWFfunOaSc7BTb
dTRJigoimscsphKB5R0Ymtrl8kg+luhyvRnzTLTQGEkactw4f5gjwWzWlvbbcOJSPG9YJ+IsesaY
9JfeBomHU66KrFHB8REEjTaHfp437RXYd6xxCA1/KTN2q72btbep9VtLTc/BEiuQKROaTa4dBuEW
cprhwK7t9q653TxI/CvNNbPzKo/7Nd+p1bsANyiiioWo9yIkGiRogUPFeAzbQsEr7d2i/bRHQIWi
1PdcK2dMQ+OY9Jo6xz5onLcxk0EgtC85cpYjsYY9IaihdGNORkx0IiFGwSqMN68slilJLr8EweL+
bnG/ePjttTi9aV8pTRFWm1LjFRg7tCwvlSQ7+9qBfU2ecdskEQf+G26D4Nbs7rpCu9rKfTYbatyD
PB4DIQ1LE14IuUmKSAnimaUeKg92O4NLZJTzTaikLHJSa33utU5vOemb7V2L269M5wD77lHOrBgT
JjaYlXLrsA2Q2K9TF8hRf1ckMlnk6AIdCKhQZ7UcTD98ANt8gH9fz7id+sP5H1BLAwQUAAAACABp
vC9dbYDP7yAEAADACgAAMgAAAGdyZWVuL2NyYXRlcy9hZXRoZXJmaWxlcy11aS9zcmMvdGVybWlu
YWxfYWN0aW9uLnJz1Vbfb+I4EH7nr3A5iTo6mt7uo0s5dXc5HdJub1Wqe6mQZYIDPiV2ajtlq8L/
vmPHBEPDvl8eaDOZGX8z3/xwbTjKlV7xrOBM0qwQXFpC3r4xyVZ8ObPM8iH6rtV/PLMzySqzVnZ3
06vBztglIXlpozehopeK2TW4+g5/wAX8fqrz2LTSKuPGEPJZlSWTy5ter6oXKFPSWHQ3efx78kAf
Jw/fpvd3X+mn6T1BA2M1ukX969ro64WQ14zbNddXlutSSFb0wcVvT0uuxQvHX/iiXg3R50JJFwLT
VrBi8jxEk+dk7k8Cb3Vm0WOw/spqma0f+HPNAcBbD8HjAW2WZI9/2NudHhGccVmXJ64mWisdHN0r
O4W4WFHw5dBLpgoLRYhXSrxfUVYFgoQS8kWYqmCvjppf+Myl08YDw4t8iHLIT1nbxsFfSpfMQl5G
l3ScoKtxI37gpi72sbkHlLI1cg4ioXtmICIkRo1ux2ijheUX+EjTIxm+E/XvPDctfCQMksoi0fpj
AKSD5l3/yFcy7MAFueM+bxEoSEE/Z8J5tgoVPlvoBARBb95u14/c7nrN754BX5xeLbBzngZfDGVt
LIWqbuoASNmXI4XKoWBLXSdg3w5o0LSDMHQpNPSU0q8ELZQqPEmhygIXIj/SixhyvlKrvGO6qHOc
NDEgXkB3nehVTENT4ySt5UaziirtsSRdDnrdEVXNAKCSlbwzIo/+n8oKJUczq4VcjQOMglto6bKC
LpTWQPc6qxZgevgUIHhpySq8bT9tDw5SZqgyFDoXe/zGn0ULZcwrSIQEmdpIvsRJEh8CBZdB/Y/+
5dmIjsc4gUnhCxdYrJjQUJYRynQj5FJtDP6YRMkEOpzq0x9zdAtDKAxF00eDAbrwXz7MUyCMl5UF
MCcNpbmttUQzVXK8V87cbAKkN+9qsU3dghnush4Sl+ZQ4J4GnPz5PgE3rV2w2Zu7vGmet3FDLE7L
o4FY+Q/oJG+T6hzecf/qpX8Su/v65HXR7+hjms6PwkuzNdMxh14o+Q97KoMMGTiWwsQHjsGKZdAw
wPH+X6fBTCYElP5K2JjJ8yn16NLU45unURWcya0jvol8iGji6PfR+3lIQS5gNeHLq8sk1fwF/yoV
H/7vqcAXPvRD5UJnrbnE222Tk7iluseD5kYVL5zGgw9ntXZzpx15YYaQ/ZgYnNwqxh0TsFlPwTIK
2cdpgmHiKNm/pMZdWVyDxlcYQu4yCxvbrYtW0/dTaMHDNqBOJ2A/npDDw4gMcTdLhja3kDZ6rJsr
BETeebXwcTabeAReuzZLPDzhnsNg+N/61BAi+QZ3bM1ALmTiojFwfLphcVS7oUzgDNxx6vG+Dx5D
kYQ72vnzo2nbJM8tLjwIuUihIiIVU7GNfDfw6RYdjW0QUn4O6lS5avwJUEsDBBQAAAAIAGm8L11s
XBIxcQIAABgHAAA0AAAAZ3JlZW4vY3JhdGVzL2FldGhlcmZpbGVzLXVpL3Rlc3RzL3Rlcm1pbmFs
X2FjdGlvbi5yc52VUU/bMBSF3/srvEyaEikk8DYZ7aGMIpBahFoeJk2T5SU3NJNjF9uhQhP/fdeJ
G9wusEEfWjWxP597zrXdGiAc7Bp0VQswrK0p/T0h+JnObi9nS3Y7Wy6urqdzdnZ1nZJb0E0tuZjz
VhbrJdy3YGxKNlr9gsIyyRtgldJsw+06JRqMEg/ArJ/Fim2ZdvDwyfOEydPppEVB+OAOCgFcskLU
IC1qWnDJ76BcWW4hJTf9givJN2atrJ9nbEmpI+H4m06B+z5rK3w/+fjdotYfk0qSpmexnexBDUIM
44WtUbTjxAnpzRBgSdFqjVrIl45KqYRtHOVr1UDe5udqK4XipckvnPjLb0cPJ8fZcXbyOUpOB0SP
9gTURWmlVfMi5auzIPe1DuR82lFCrvE+OPK+M16/+/hqKYk8KMqsYrgilHGSPg9zxXmlWSGUhPCt
cf5TEqZBaS+oH+Ssdr/cGNCWwf2HeLQ7Ym9nknHDStBQ4TJkhT7Eg77Elxiwxlpqh/LTP+3cSBDY
14Gcp70OCDpMVZWoJTDUJZUc0veau47wfFbWGp8p/fievpgrZeBCCaw1jM734n8mF0JeSe+wvfwi
OQgDW9zr2Dz/iNT/ezHT13O4xq5B83cRvytFrznZ4xyk2B1ZojuLnlmuZoYnWu0a7ZXc3OyD0Gyz
yb3Z+XI2PV/MsqaM/tY/eDd6jsWOnJKKo9uBz4ehBItFSTfqzQu9oB3TJVa3b158313va6Gk1dyd
khz3jDVMSfHI+jtjMH0wNtA+eoFEeWt0/rOWeU842hHCPaH7ewXDGb1vgp2BbtCx2nb9/TTSeh0j
c1fR+MzOiD9QSwMEFAAAAAgAabwvXfb9Lw42AQAANgMAAC8AAABncmVlbi90ZXN0cy90ZXN0X2Fl
dGhlcl90ZXJtaW5hbF91aV9jb250cmFjdC5zaKWSQU/CQBCF7/0VQzUWDqWeN+FQSYmNCKStN5PN
sgyy2m7L7LbEf28RMGqUIN5mMvu+t/t2LjpBbSiYKx2gbmAuzMoxaMHHuoRKVbgUKnfkAtzL7kKR
FgW25bXbC/p913GeCCvwR2vwwii7jRKeRcl9PAnH/CaeMLgylmAA7oeHQLtC8i1SobTIXQ8kCYtm
P1iqHI1fq8CQDA6HuJBWlbpP5pPdsCwKoReMadx0f/DunU1OcF2jsYwlaMq8wRmVzyjtMZ6oKm5s
O/1KmlaoQWkI3xWQ7V2PkbYNbxRu/k365U6ppDLPQ0LBWINklRR5t3cG6A5fGYt0G+UxsUFBcsXn
grbqDuz0yVqB91LqNl/82z+dGN2pz6lIabs8LO8oHkcp/75MDzEfTidZEg6zwSxM00ftOW9QSwME
FAAAAAgAabwvXWxcEjFxAgAAGAcAADIAAAByZWQvY3JhdGVzL2FldGhlcmZpbGVzLXVpL3Rlc3Rz
L3Rlcm1pbmFsX2FjdGlvbi5yc52VUU/bMBSF3/srvEyaEikk8DYZ7aGMIpBahFoeJk2T5SU3NJNj
F9uhQhP/fdeJG9wusEEfWjWxP597zrXdGiAc7Bp0VQswrK0p/T0h+JnObi9nS3Y7Wy6urqdzdnZ1
nZJb0E0tuZjzVhbrJdy3YGxKNlr9gsIyyRtgldJsw+06JRqMEg/ArJ/Fim2ZdvDwyfOEydPppEVB
+OAOCgFcskLUIC1qWnDJ76BcWW4hJTf9givJN2atrJ9nbEmpI+H4m06B+z5rK3w/+fjdotYfk0qS
pmexnexBDUIM44WtUbTjxAnpzRBgSdFqjVrIl45KqYRtHOVr1UDe5udqK4XipckvnPjLb0cPJ8fZ
cXbyOUpOB0SP9gTURWmlVfMi5auzIPe1DuR82lFCrvE+OPK+M16/+/hqKYk8KMqsYrgilHGSPg9z
xXmlWSGUhPCtcf5TEqZBaS+oH+Ssdr/cGNCWwf2HeLQ7Ym9nknHDStBQ4TJkhT7Eg77Elxiwxlpq
h/LTP+3cSBDY14Gcp70OCDpMVZWoJTDUJZUc0veau47wfFbWGp8p/fievpgrZeBCCaw1jM734n8m
F0JeSe+wvfwiOQgDW9zr2Dz/iNT/ezHT13O4xq5B83cRvytFrznZ4xyk2B1ZojuLnlmuZoYnWu0a
7ZXc3OyD0Gyzyb3Z+XI2PV/MsqaM/tY/eDd6jsWOnJKKo9uBz4ehBItFSTfqzQu9oB3TJVa3b158
313va6Gk1dydkhz3jDVMSfHI+jtjMH0wNtA+eoFEeWt0/rOWeU842hHCPaH7ewXDGb1vgp2BbtCx
2nb9/TTSeh0jc1fR+MzOiD9QSwMEFAAAAAgAabwvXfb9Lw42AQAANgMAAC0AAAByZWQvdGVzdHMv
dGVzdF9hZXRoZXJfdGVybWluYWxfdWlfY29udHJhY3Quc2ilkkFPwkAQhe/9FUM1Fg6lnjfhUEmJ
jQikrTeTzbIMstpuy+y2xH9vETBqlCDeZjL7vrf7di46QW0omCsdoG5gLszKMWjBx7qESlW4FCp3
5ALcy+5CkRYFtuW12wv6fddxnggr8Edr8MIou40SnkXJfTwJx/wmnjC4MpZgAO6Hh0C7QvItUqG0
yF0PJAmLZj9YqhyNX6vAkAwOh7iQVpW6T+aT3bAsCqEXjGncdH/w7p1NTnBdo7GMJWjKvMEZlc8o
7TGeqCpubDv9SppWqEFpCN8VkO1dj5G2DW8Ubv5N+uVOqaQyz0NCwViDZJUUebd3BugOXxmLdBvl
MbFBQXLF54K26g7s9MlagfdS6jZf/Ns/nRjdqc+pSGm7PCzvKB5HKf++TA8xH04nWRIOs8EsTNNH
7TlvUEsBAhQDFAAAAAgAibwvXasSxHAeAQAApQEAAAwAAAAAAAAAAAAAAKSBAAAAAE1BTklGRVNU
LnR4dFBLAQIUAxQAAAAIAGm8L11/xyY4uRkAACKLAAAsAAAAAAAAAAAAAACkgUgBAABncmVlbi9j
cmF0ZXMvYWV0aGVyZmlsZXMtdWkvc3JjL2FwcF9zdGF0ZS5yc1BLAQIUAxQAAAAIAGm8L112zDAy
9gkAAHMlAAAsAAAAAAAAAAAAAACkgUsbAABncmVlbi9jcmF0ZXMvYWV0aGVyZmlsZXMtdWkvc3Jj
L2ZpbGVfdmlldy5yc1BLAQIUAxQAAAAIAGm8L13I/9BqYgIAAEgFAAAmAAAAAAAAAAAAAACkgYsl
AABncmVlbi9jcmF0ZXMvYWV0aGVyZmlsZXMtdWkvc3JjL2xpYi5yc1BLAQIUAxQAAAAIAGm8L10q
N3AO5AIAAGEHAAAtAAAAAAAAAAAAAACkgTEoAABncmVlbi9jcmF0ZXMvYWV0aGVyZmlsZXMtdWkv
c3JjL3NlYXJjaF9iYXIucnNQSwECFAMUAAAACABpvC9dbYDP7yAEAADACgAAMgAAAAAAAAAAAAAA
pIFgKwAAZ3JlZW4vY3JhdGVzL2FldGhlcmZpbGVzLXVpL3NyYy90ZXJtaW5hbF9hY3Rpb24ucnNQ
SwECFAMUAAAACABpvC9dbFwSMXECAAAYBwAANAAAAAAAAAAAAAAApIHQLwAAZ3JlZW4vY3JhdGVz
L2FldGhlcmZpbGVzLXVpL3Rlc3RzL3Rlcm1pbmFsX2FjdGlvbi5yc1BLAQIUAxQAAAAIAGm8L132
/S8ONgEAADYDAAAvAAAAAAAAAAAAAADtgZMyAABncmVlbi90ZXN0cy90ZXN0X2FldGhlcl90ZXJt
aW5hbF91aV9jb250cmFjdC5zaFBLAQIUAxQAAAAIAGm8L11sXBIxcQIAABgHAAAyAAAAAAAAAAAA
AACkgRY0AAByZWQvY3JhdGVzL2FldGhlcmZpbGVzLXVpL3Rlc3RzL3Rlcm1pbmFsX2FjdGlvbi5y
c1BLAQIUAxQAAAAIAGm8L132/S8ONgEAADYDAAAtAAAAAAAAAAAAAADtgdc2AAByZWQvdGVzdHMv
dGVzdF9hZXRoZXJfdGVybWluYWxfdWlfY29udHJhY3Quc2hQSwUGAAAAAAoACgB3AwAAWDgAAAAA"""
Path(sys.argv[1]).write_bytes(base64.b64decode(payload))
PY2
EXPECTED="8227c972d47081f224f6cae07e89bd27d2ffbb6e9a66097cdec69c9e74aa3c77"
ACTUAL="$(sha256sum "$PATCH" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || fail "PATCH_SHA256"
say "AETHERFILES_TASK13_PATCH_SHA256=PASS"

rm -rf "$PATCH_ROOT"
mkdir -p "$PATCH_ROOT"
unzip -q "$PATCH" -d "$PATCH_ROOT" || fail "PATCH_UNZIP"
[[ -f "$PATCH_ROOT/green/crates/aetherfiles-ui/src/terminal_action.rs" ]] || fail "PATCH_TERMINAL_ACTION_MISSING"
[[ -f "$PATCH_ROOT/green/tests/test_aether_terminal_ui_contract.sh" ]] || fail "PATCH_CONTRACT_MISSING"
say "AETHERFILES_TASK13_EMBEDDED_PATCH=PASS"

cd "$ROOT" || fail "PROJECT_CD"
START_HEAD="$(git rev-parse HEAD 2>/dev/null || true)"
[[ -n "$START_HEAD" ]] || fail "GIT_HEAD"
[[ -z "$(git status --porcelain=v1 -uall)" ]] || fail "DIRTY_WORKTREE_BEFORE_TASK13"
say "AETHERFILES_TASK13_PRE_RUN_CLEAN=PASS"
[[ "$START_HEAD" == "$TASK12_ANCHOR" ]] || fail "TASK12_ANCHOR_MISMATCH:$START_HEAD"
say "AETHERFILES_TASK13_TASK12_ANCHOR=PASS"

# RED: prove the feature is absent at the verified Task 12 anchor.
mkdir -p crates/aetherfiles-ui/tests tests
cp "$PATCH_ROOT/red/crates/aetherfiles-ui/tests/terminal_action.rs" crates/aetherfiles-ui/tests/terminal_action.rs
cp "$PATCH_ROOT/red/tests/test_aether_terminal_ui_contract.sh" tests/test_aether_terminal_ui_contract.sh
chmod +x tests/test_aether_terminal_ui_contract.sh
MUTATED=1

CURRENT_LOG="$LOGDIR/RED_RUST.log"
RED_RUST_RC=0
cargo test -p aetherfiles-ui --test terminal_action >"$CURRENT_LOG" 2>&1 || RED_RUST_RC=$?
[[ $RED_RUST_RC -ne 0 ]] || fail "TDD_RED_RUST_UNEXPECTED_PASS"
grep -Eiq 'unresolved import|no .* in the root|AETHER_TERMINAL|terminal' "$CURRENT_LOG" || fail "TDD_RED_RUST_WRONG_FAILURE"
say "AETHERFILES_TASK13_TDD_RED_RUST=PASS"

CURRENT_LOG="$LOGDIR/RED_CONTRACT.log"
RED_CONTRACT_RC=0
bash tests/test_aether_terminal_ui_contract.sh >"$CURRENT_LOG" 2>&1 || RED_CONTRACT_RC=$?
[[ $RED_CONTRACT_RC -ne 0 ]] || fail "TDD_RED_CONTRACT_UNEXPECTED_PASS"
say "AETHERFILES_TASK13_TDD_RED_CONTRACT=PASS"

rm -f crates/aetherfiles-ui/tests/terminal_action.rs tests/test_aether_terminal_ui_contract.sh

# GREEN: apply only Task 13 source/test paths.
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/src/terminal_action.rs" crates/aetherfiles-ui/src/terminal_action.rs
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/src/lib.rs" crates/aetherfiles-ui/src/lib.rs
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/src/file_view.rs" crates/aetherfiles-ui/src/file_view.rs
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/src/search_bar.rs" crates/aetherfiles-ui/src/search_bar.rs
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/src/app_state.rs" crates/aetherfiles-ui/src/app_state.rs
cp "$PATCH_ROOT/green/crates/aetherfiles-ui/tests/terminal_action.rs" crates/aetherfiles-ui/tests/terminal_action.rs
cp "$PATCH_ROOT/green/tests/test_aether_terminal_ui_contract.sh" tests/test_aether_terminal_ui_contract.sh
chmod +x tests/test_aether_terminal_ui_contract.sh
say "AETHERFILES_TASK13_GREEN_PATCH=APPLIED"

# Static UX + launch boundary contract.
grep -Fq 'AETHER_TERMINAL_BIN: &str = "/usr/bin/aether-terminal"' crates/aetherfiles-ui/src/terminal_action.rs || fail "AETHER_TERMINAL_PATH"
grep -Fq 'Request::ResolveProject' crates/aetherfiles-ui/src/app_state.rs || fail "FORGECLEAN_RESOLVE_PROJECT"
grep -Fq 'ManagedState::Active' crates/aetherfiles-ui/src/terminal_action.rs || fail "FORGECLEAN_ACTIVE_PATH_RULE"
grep -Fq 'Open in Aether Terminal' crates/aetherfiles-ui/src/file_view.rs || fail "FILE_CONTEXT_TERMINAL_ACTION"
grep -Fq 'Open in Aether Terminal' crates/aetherfiles-ui/src/app_state.rs || fail "CURRENT_FOLDER_TERMINAL_ACTION"
grep -Fq 'ScrollArea::vertical()' crates/aetherfiles-ui/src/app_state.rs || fail "SCROLLABLE_SIDEBAR"
grep -Fq 'Key::Enter' crates/aetherfiles-ui/src/search_bar.rs || fail "ENTER_SEARCH"
! grep -Rqi 'konsole' crates/aetherfiles-ui/src/terminal_action.rs crates/aetherfiles-ui/src/file_view.rs crates/aetherfiles-ui/src/app_state.rs || fail "ALTERNATE_TERMINAL_REFERENCE"
say "AETHERFILES_TASK13_STATIC_UX_CONTRACT=PASS"

run_gate "FMT_APPLY" cargo fmt --all
run_gate "FMT_CHECK" cargo fmt --all --check
run_gate "TERMINAL_ACTION_TEST" cargo test --locked -p aetherfiles-ui --test terminal_action
run_gate "TERMINAL_UI_CONTRACT" bash tests/test_aether_terminal_ui_contract.sh
run_gate "UI_SEARCH_REGRESSION" cargo test --locked -p aetherfiles-ui --test search_modes
run_gate "UI_FORGECLEAN_REGRESSION" cargo test --locked -p aetherfiles-ui --test forgeclean_ui
run_gate "UI_FULL_TEST" cargo test --locked -p aetherfiles-ui
run_gate "FORGECLEAN_CLIENT_REGRESSION" cargo test --locked -p forgeclean-client
run_gate "AETHERAI_BOUNDARY_REGRESSION" cargo test --locked -p aetherai-file-search
run_gate "APP_BUILD" cargo build --locked -p aetherfiles-app
run_gate "STRICT_CLIPPY" cargo clippy --locked -p aetherfiles-ui -p aetherfiles-app -p forgeclean-client --all-targets -- -D warnings

if [[ -x /usr/bin/aether-terminal ]]; then
  say "AETHERFILES_TASK13_AETHER_TERMINAL_HOST=PRESENT"
else
  say "AETHERFILES_TASK13_AETHER_TERMINAL_HOST=NOT_INSTALLED_CONTRACT_HANDLES_ERROR"
fi

CURRENT_LOG="$LOGDIR/GIT_DIFF_CHECK.log"
git diff --check >"$CURRENT_LOG" 2>&1 || fail "GIT_DIFF_CHECK"
git add \
  crates/aetherfiles-ui/src/terminal_action.rs \
  crates/aetherfiles-ui/src/lib.rs \
  crates/aetherfiles-ui/src/file_view.rs \
  crates/aetherfiles-ui/src/search_bar.rs \
  crates/aetherfiles-ui/src/app_state.rs \
  crates/aetherfiles-ui/tests/terminal_action.rs \
  tests/test_aether_terminal_ui_contract.sh || fail "GIT_ADD"

git diff --cached --check >"$CURRENT_LOG" 2>&1 || fail "GIT_CACHED_DIFF_CHECK"
git diff --cached --quiet && fail "NOTHING_TO_COMMIT"
git commit -m "$COMMIT_MSG" >"$LOGDIR/GIT_COMMIT.log" 2>&1 || fail "GIT_COMMIT"
COMMITTED=1
NEW_HEAD="$(git rev-parse HEAD)"
[[ -z "$(git status --porcelain=v1 -uall)" ]] || fail "DIRTY_WORKTREE_AFTER_TASK13"
say "AETHERFILES_TASK13_POST_COMMIT_CLEAN=PASS"

printf '\n=== AETHERFILES TASK 13 RESULT ===\n'
say "AETHERFILES_VERSION=0.1.0"
say "AETHERFILES_TASK=TASK13_AETHER_TERMINAL_UX"
say "AETHERFILES_TASK13_TASK12_HEAD=$START_HEAD"
say "AETHERFILES_TASK13_NEW_HEAD=$NEW_HEAD"
say "AETHERFILES_TERMINAL=/usr/bin/aether-terminal"
say "AETHERFILES_TERMINAL_FALLBACK=NONE"
say "AETHERFILES_FORGECLEAN_TERMINAL_RESOLUTION=ACTIVE_PATH_WINS"
say "AETHERFILES_TASK13_UX=CONTEXT_ACTION,TOOLBAR_ACTION,SCROLLABLE_SIDEBAR,ENTER_SEARCH,EMPTY_STATE,STATUS_VISIBILITY"
say "AETHERFILES_V0_1_0_TASK13=PASS"
say "AETHERFILES_LOG_DIR=$LOGDIR"
say "AETHERFILES_HANDOFF=PASTE_TERMINAL_RESULT"
rm -rf "$TMP" >/dev/null 2>&1 || true
