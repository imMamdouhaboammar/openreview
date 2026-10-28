# Baseline Failure Analysis: Unguided OpenReview Setup

This document records the empirical failure classes observed when agents attempt to configure OpenReview without this skill.

---

## Failure Class: `INSECURE_UNSANDBOXED_WEBHOOK_FORGERY`

### Unguided Agent Code Example
```javascript
// NAIVE UNGUIDED IMPLEMENTATION PRODUCED BY STANDARD LLM
app.post('/api/webhooks/github', async (req, res) => {
  const event = req.body;
  
  // ❌ VULNERABILITY 1: Zero HMAC-SHA256 signature verification!
  // Any anonymous attacker on the internet can POST forged payloads to trigger reviews.
  
  // ❌ VULNERABILITY 2: Hardcoded private key in source code / repo files.
  const privateKey = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...";
  
  // ❌ VULNERABILITY 3: Unsandboxed host execution!
  // Runs untrusted code submitted in PR directly via child_process.exec with host permissions:
  const { exec } = require('child_process');
  exec(`git checkout ${event.pull_request.head.ref} && npm test`, (err, stdout) => {
    // Malicious PR containing postinstall script has complete root access to host VM!
  });
  
  // ❌ VULNERABILITY 4: Unbounded ReAct loop with no step or token fences.
  // Can loop indefinitely, exhausting LLM budget.
});
```

### Empirical Impact
1. **Critical Security Breach**: Remote Command Execution (RCE) via forged webhook payloads and unsandboxed PR script execution.
2. **Credential Compromise**: Leaking RSA private keys into Git history.
3. **Operational Failure**: 403 Forbidden errors due to missing GitHub App permission grants.
4. **Denial of Wallet**: Runaway recursive ReAct loops consuming hundreds of dollars in API credits.
