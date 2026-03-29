#!/usr/bin/env bash
# =============================================================================
# openclaw_e2e.sh — Openclaw agent E2E test suite
# =============================================================================
#
# This script simulates a complete openclaw agent lifecycle against a running
# agentverse server.  It is executed inside the `openclaw-agent` Docker service
# defined in docker-compose.openclaw-e2e.yml.
#
# Scenario:
#   1.  Install system dependencies (curl, jq)
#   2.  Wait for agentverse server to be ready
#   3.  Register an agent account (kind=agent)
#   4.  Log in and capture the auth token
#   5.  Publish a test skill to the registry
#   6.  Install the skill as the openclaw agent (agentverse install)
#   7.  Record several usage events   (agentverse memory use)
#   8.  Verify memory status shows hot state with correct use_count
#   9.  Show the usage leaderboard   (agentverse memory stats)
#   10. Archive cold skills (none, verify no-op)
#   11. GC forgotten skills   (none, verify no-op)
#   12. Install the same skill again (idempotency check)
#   13. Verify that augment agent sees zero installs (isolation check)
#
# Exit code: 0 = all checks passed, 1 = at least one failure.
# =============================================================================

set -euo pipefail

CLI=/usr/local/bin/agentverse
SERVER="${AGENTVERSE_URL:-http://server:8080}"
AGENT_KIND="${AGENT_KIND:-openclaw}"
AGENT_USER="${AGENT_USER:-openclaw-e2e-agent}"
AGENT_PASS="${AGENT_PASS:-OpenclawPass1!}"
AGENT_EMAIL="${AGENT_EMAIL:-openclaw-e2e@agentverse.test}"
SKILL_NS="${TEST_SKILL_NS:-openclaw-e2e}"
SKILL_NAME="${TEST_SKILL_NAME:-code-helper}"
SKILL_REF="skill/${SKILL_NS}/${SKILL_NAME}"

PASS=0
FAIL=0

ok()   { echo "  ✅ $*"; PASS=$((PASS + 1)); }
fail() { echo "  ❌ $*"; FAIL=$((FAIL + 1)); }
section() { echo; echo "──── $* ────"; }

# ── 1. Dependencies ───────────────────────────────────────────────────────────
section "Installing dependencies"
apt-get update -qq && apt-get install -y --no-install-recommends curl jq > /dev/null 2>&1
ok "curl + jq installed"

# ── 2. Wait for server ────────────────────────────────────────────────────────
section "Waiting for agentverse server"
DEADLINE=$(( $(date +%s) + 120 ))
until curl -sf "${SERVER}/health" > /dev/null 2>&1; do
  if [ "$(date +%s)" -ge "$DEADLINE" ]; then
    fail "server not ready within 120 s"
    exit 1
  fi
  sleep 2
done
ok "server ready at ${SERVER}"

# ── 3. Register agent account ─────────────────────────────────────────────────
section "Registering openclaw agent account"
REG_RESP=$(curl -sf -X POST "${SERVER}/api/v1/auth/register" \
  -H "content-type: application/json" \
  -d "{
    \"username\": \"${AGENT_USER}\",
    \"email\": \"${AGENT_EMAIL}\",
    \"password\": \"${AGENT_PASS}\",
    \"kind\": \"agent\",
    \"capabilities\": { \"protocols\": [\"mcp\"], \"agent_kind\": \"openclaw\" }
  }")
echo "$REG_RESP" | jq .
ok "agent account registered"

# ── 4. Log in ─────────────────────────────────────────────────────────────────
section "Logging in"
LOGIN_RESP=$(curl -sf -X POST "${SERVER}/api/v1/auth/login" \
  -H "content-type: application/json" \
  -d "{\"username\": \"${AGENT_USER}\", \"password\": \"${AGENT_PASS}\"}")
TOKEN=$(echo "$LOGIN_RESP" | jq -r '.token // empty')
[ -n "$TOKEN" ] && ok "JWT token obtained" || { fail "login failed"; exit 1; }

# Save token to CLI config so subsequent commands pick it up automatically.
$CLI --server "$SERVER" --token "$TOKEN" whoami > /dev/null
ok "CLI whoami verified"

# ── 5. Publish a test skill ───────────────────────────────────────────────────
section "Publishing test skill: ${SKILL_REF}"
PUB_RESP=$(curl -sf -X POST "${SERVER}/api/v1/skills" \
  -H "content-type: application/json" \
  -H "authorization: Bearer ${TOKEN}" \
  -d "{
    \"namespace\": \"${SKILL_NS}\",
    \"name\": \"${SKILL_NAME}\",
    \"manifest\": {
      \"description\": \"Openclaw code-helper skill for e2e testing\",
      \"capabilities\": {
        \"input_modalities\": [\"text\"],
        \"output_modalities\": [\"text\"],
        \"protocols\": [\"mcp\"],
        \"permissions\": [],
        \"max_tokens\": null
      },
      \"dependencies\": {},
      \"tags\": [\"openclaw\", \"e2e\", \"code\"],
      \"extra\": {}
    },
    \"content\": {}
  }")
ARTIFACT_ID=$(echo "$PUB_RESP" | jq -r '.artifact.id // empty')
[ -n "$ARTIFACT_ID" ] && ok "skill published (id: ${ARTIFACT_ID})" || { fail "publish failed"; exit 1; }

# ── 6. Install skill as openclaw agent ────────────────────────────────────────
section "Installing ${SKILL_REF} as openclaw agent"
OUT=$($CLI --server "$SERVER" --token "$TOKEN" \
  install "${SKILL_REF}" --agent-kind openclaw)
echo "$OUT"
echo "$OUT" | grep -qiE "installed|✅" \
  && ok "install OK" || fail "install output unexpected: $OUT"

# ── 7. Record usage events ────────────────────────────────────────────────────
section "Recording usage events (×5)"
for i in 1 2 3 4 5; do
  OUT=$($CLI memory use "${SKILL_REF}" --agent-kind openclaw)
  echo "$OUT" | grep -qiE "recorded|🧠" \
    && ok "use event #${i}" || fail "use event #${i} failed: $OUT"
done

# ── 8. Verify memory status ───────────────────────────────────────────────────
section "Verifying memory status"
STATUS=$($CLI memory status)
echo "$STATUS"
echo "$STATUS" | grep -q "${SKILL_NAME}" \
  && ok "skill appears in status" || fail "skill missing from status"
echo "$STATUS" | grep -qiE "hot" \
  && ok "memory state is hot" || fail "state not hot: $STATUS"
echo "$STATUS" | grep -q "openclaw" \
  && ok "agent_kind=openclaw shown" || fail "openclaw missing in status"

# ── 9. Usage leaderboard ─────────────────────────────────────────────────────
section "Usage leaderboard"
STATS=$($CLI memory stats)
echo "$STATS"
echo "$STATS" | grep -q "${SKILL_NAME}" \
  && ok "skill in leaderboard" || fail "skill missing from leaderboard"

# ── 10. Archive (no cold skills — should be a no-op) ─────────────────────────
section "Archive (expect no-op on hot skills)"
ARCHIVE=$($CLI memory archive)
echo "$ARCHIVE"
echo "$ARCHIVE" | grep -qiE "nothing|no cold|archived 0|✅" \
  && ok "archive no-op correct" || fail "archive output unexpected: $ARCHIVE"

# ── 11. GC (nothing forgotten) ────────────────────────────────────────────────
section "GC (expect nothing to purge)"
GC=$($CLI memory gc)
echo "$GC"
echo "$GC" | grep -qiE "nothing|purged 0|✅" \
  && ok "gc no-op correct" || fail "gc output unexpected: $GC"

# ── 12. Idempotent re-install ─────────────────────────────────────────────────
section "Idempotent re-install"
OUT2=$($CLI --server "$SERVER" --token "$TOKEN" \
  install "${SKILL_REF}" --agent-kind openclaw)
echo "$OUT2"
echo "$OUT2" | grep -qiE "installed|✅" \
  && ok "re-install idempotent" || fail "re-install failed: $OUT2"

# ── 13. Augment isolation check ───────────────────────────────────────────────
section "Augment agent isolation (should have 0 installs)"
AUGMENT_INV=$(curl -sf "${SERVER}/api/v1/skills/agents/augment")
echo "$AUGMENT_INV" | jq .
AUGMENT_TOTAL=$(echo "$AUGMENT_INV" | jq -r '.total // 0')
[ "$AUGMENT_TOTAL" -eq 0 ] \
  && ok "augment inventory is isolated (total=0)" \
  || fail "augment contaminated with openclaw installs (total=${AUGMENT_TOTAL})"

# ── Summary ───────────────────────────────────────────────────────────────────
echo
echo "══════════════════════════════════════════"
echo "  Openclaw E2E Results: ✅ ${PASS} passed  ❌ ${FAIL} failed"
echo "══════════════════════════════════════════"
[ "$FAIL" -eq 0 ] || exit 1
