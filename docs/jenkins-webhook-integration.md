# Testing the PR webhook flow with Jenkins

This guide wires a real Jenkins pipeline to the app's webhooks so you can watch
a pull request drive CI end to end. It is a **local test harness**, not a
production CI design.

The flow being tested:

```
open/push/merge PR → activity event → webhook POST (HMAC signed)
                   → Jenkins Generic Webhook Trigger → pipeline builds the branch
                   → optional comment posted back to the PR
```

Two hosting options are described: **Docker Compose** (simplest) and a local
**Kubernetes/K3s** cluster ("Kiri" in the original request was read as K8s/K3s).
Both run Jenkins and the app on the same machine; adapt the hostnames if your
app runs elsewhere.

## What the app actually sends

`GET /api/webhook-events` lists the subscribable kinds. For this test subscribe
to `pr.opened`, `pr.merged` and `push` (or `*`).

Every delivery is a `POST` with `Content-Type: application/json` and these
headers:

| Header | Value |
|---|---|
| `X-GitHub-Event` | `pr.opened`, `push`, `pr.merged`, … |
| `X-GitHub-Delivery` | delivery row id (use for dedupe) |
| `X-Hub-Signature-256` | `sha256=<hex HMAC-SHA256 of the raw body>` with the webhook secret |

Body shape:

```json
{
  "event": "pr.opened",
  "created_at": "2026-11-18T...",
  "repository": { "id": 1, "owner": "subbu", "name": "api", "private": false },
  "sender": { "id": 3, "username": "subbu" },
  "payload": { "number": 7 }
}
```

For `push`, `payload.branch` holds the pushed branch. PR events only carry the
PR number, so the pipeline must look up `head_ref` through the API (shown
below).

## 1. Start Jenkins (Docker Compose)

```yaml
# docker-compose.jenkins.yml
services:
  jenkins:
    image: jenkins/jenkins:lts-jdk17
    ports:
      - "8080:8080"
      - "50000:50000"
    volumes:
      - jenkins_home:/var/jenkins_home
    environment:
      JAVA_OPTS: "-Djenkins.install.runSetupWizard=false"
    # Linux only: lets the container reach the app running on the host.
    extra_hosts:
      - "host.docker.internal:host-gateway"
volumes:
  jenkins_home:
```

```sh
docker compose -f docker-compose.jenkins.yml up -d
docker compose -f docker-compose.jenkins.yml logs -f jenkins
```

Get the initial admin password from
`docker compose exec jenkins cat /var/jenkins_home/secrets/initialAdminPassword`,
log in at `http://localhost:8080`, and install:

- **Generic Webhook Trigger**
- **Pipeline**
- **Git**

Create a Pipeline job, e.g. `pr-webhook-test`, and tick *Trigger builds
remotely* only if you also want manual starts. The token below is the Generic
Webhook Trigger token, configured in the job.

## 2. Verify the signature before Jenkins (recommended)

Jenkins' Generic Webhook Trigger does **not** verify `X-Hub-Signature-256` by
itself. For a faithful test, put a tiny verifier in front of it. Save this as
`verify_hook.py` on the host:

```python
import hashlib, hmac, http.server, urllib.request, os

SECRET = os.environ["WEBHOOK_SECRET"].encode()   # the whsec_… from webhook creation
JENKINS = "http://127.0.0.1:8080/generic-webhook-trigger/invoke?token=LOCALTOKEN"

class Hook(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        expected = "sha256=" + hmac.new(SECRET, body, hashlib.sha256).hexdigest()
        if not hmac.compare_digest(self.headers.get("X-Hub-Signature-256", ""), expected):
            self.send_response(401); self.end_headers(); return
        req = urllib.request.Request(
            JENKINS,
            data=body,
            headers={
                "Content-Type": "application/json",
                "X-GitHub-Event": self.headers.get("X-GitHub-Event", ""),
                "X-GitHub-Delivery": self.headers.get("X-GitHub-Delivery", ""),
            },
        )
        with urllib.request.urlopen(req) as response:
            status = response.status
        self.send_response(status); self.end_headers()

http.server.HTTPServer(("127.0.0.1", 9090), Hook).serve_forever()
```

```sh
WEBHOOK_SECRET='whsec_…from the SPA…' python3 verify_hook.py
```

The verifier must answer within a few seconds: the app's delivery worker has a
10-second timeout and the queue retries non-2xx responses up to three times.
Do **not** run the pipeline inline in this proxy — Generic Webhook Trigger
returns immediately, which is exactly what we want.

For a quick smoke test you can skip the proxy and point the webhook straight at
Jenkins, but then you are not testing signing. Note that the app's `validate_url`
accepts `http://` URLs, which is fine for localhost.

## 3. Register the webhook in the app

Log in to the SPA, open the repository, **Webhooks** tab:

- **Payload URL**: `http://127.0.0.1:9090/hook` (or `http://localhost:8080/generic-webhook-trigger/invoke?token=LOCALTOKEN` without the proxy)
- **Events**: tick `pr.opened`, `pr.merged`, `push`

Copy the signing secret from the one-time banner into `WEBHOOK_SECRET` above.
The same value is used to compute the expected HMAC.

You can do the same through the API:

```sh
curl -X POST http://localhost:5150/api/repos/subbu/api/webhooks \
  -H "Authorization: Bearer $JWT" -H 'Content-Type: application/json' \
  -d '{"url":"http://127.0.0.1:9090/hook","events":"pr.opened,pr.merged,push"}'
```

`$JWT` comes from `POST /api/auth/login` — personal access tokens are for git
Basic auth only, not for the REST API.

## 4. Jenkinsfile

Paste this into the Pipeline job. It uses the JSONPath variables from Generic
Webhook Trigger, fetches the PR's `head_ref` through the API, checks out that
branch from the app's git endpoint, and comments the result back.

```groovy
pipeline {
  agent any
  triggers {
    GenericTrigger(
      genericVariables: [
        [key: 'EVENT',      value: '$.event',               expressionType: 'JSONPath'],
        [key: 'OWNER',      value: '$.repository.owner',    expressionType: 'JSONPath'],
        [key: 'REPO',       value: '$.repository.name',     expressionType: 'JSONPath'],
        [key: 'PR_NUMBER',  value: '$.payload.number',      expressionType: 'JSONPath', defaultValue: '0'],
        [key: 'PUSH_BRANCH',value: '$.payload.branch',      expressionType: 'JSONPath', defaultValue: ''],
        [key: 'DELIVERY',   value: '$',                     expressionType: 'JSONPath'],
      ],
      token: 'LOCALTOKEN',
      causeString: 'github-on-loco $EVENT delivery $DELIVERY',
      printContributedVariables: true
    )
  }
  environment {
    APP        = 'http://host.docker.internal:5150'
    CI_LOGIN   = 'subbu@example.com'
    CI_PASS    = 'P@55ward'
  }
  stages {
    stage('Resolve branch') {
      steps {
        script {
          if (env.EVENT == 'push' && env.PUSH_BRANCH) {
            env.BRANCH = env.PUSH_BRANCH
          } else {
            def token = sh(
              script: "curl -s -X POST ${APP}/api/auth/login -H 'Content-Type: application/json' " +
                      "-d '{\"email\":\"${CI_LOGIN}\",\"password\":\"${CI_PASS}\"}' | jq -r .token",
              returnStdout: true).trim()
            def json = sh(
              script: "curl -s -H 'Authorization: Bearer ${token}' " +
                      "${APP}/api/repos/${OWNER}/${REPO}/pulls/${PR_NUMBER}",
              returnStdout: true).trim()
            env.BRANCH = new groovy.json.JsonSlurper().parseText(json).head_ref
          }
          echo "Event=${env.EVENT} branch=${env.BRANCH} repo=${OWNER}/${REPO}"
        }
      }
    }
    stage('Checkout') {
      steps {
        git branch: "${env.BRANCH}",
            url: "${APP}/${OWNER}/${REPO}.git",
            credentialsId: 'github-loco-pat'
      }
    }
    stage('Build and test') {
      steps {
        // Replace with the repository's real commands.
        sh 'echo "building ${BRANCH}" && ls -la'
      }
    }
    stage('Report back') {
      when { expression { env.PR_NUMBER != '0' } }
      steps {
        script {
          def token = sh(
            script: "curl -s -X POST ${APP}/api/auth/login -H 'Content-Type: application/json' " +
                    "-d '{\"email\":\"${CI_LOGIN}\",\"password\":\"${CI_PASS}\"}' | jq -r .token",
            returnStdout: true).trim()
          sh """curl -s -X POST ${APP}/api/repos/${OWNER}/${REPO}/pulls/${PR_NUMBER}/comments \\
            -H 'Authorization: Bearer ${token}' -H 'Content-Type: application/json' \\
            -d '{"body":"Jenkins build ${BUILD_NUMBER} passed for \`${env.BRANCH}\`."}'"""
        }
      }
    }
  }
}
```

In **Manage Jenkins → Credentials**, add `github-loco-pat` as a username/password
credential: username `subbu`, password the `glp_…` token. That is what allows the
container to clone (private repos require it; public repos clone anonymously).

## 5. Run the test

1. Create a branch, make a commit, push it.
2. Open a PR in the SPA. Watch **Webhooks → Deliveries**: a `pr.opened` row should
   go to `success`.
3. Jenkins should show a new build with the resolved `head_ref` and checkout the
   branch.
4. Push another commit to the branch — the `push` event triggers a new build
   with `payload.branch`.
5. Merge the PR. The worker emits `pr.merged`, which triggers one more build,
   and the PR conversation gains the Jenkins comment.

## Option B: local Kubernetes / K3s

If "Kiri" meant a local Kubernetes cluster, the same pieces run as workloads.
The important difference is DNS: the app is reachable as a Service, not via
`host.docker.internal`.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: github-loco
spec:
  selector: { app: github-loco }
  ports: [{ port: 5150, targetPort: 5150 }]
---
apiVersion: v1
kind: Service
metadata:
  name: jenkins
spec:
  selector: { app: jenkins }
  ports: [{ port: 8080, targetPort: 8080 }]
```

Run the app and Jenkins in the same namespace and set the Jenkinsfile `APP` to
`http://github-loco:5150`. Point the webhook at
`http://jenkins.<namespace>.svc.cluster.local:8080/generic-webhook-trigger/invoke?token=LOCALTOKEN`
(or at the verifier Service if you keep the HMAC proxy). Expose Jenkins with an
Ingress or `kubectl port-forward svc/jenkins 8080:8080` for the UI. Keep the
signing secret in a Kubernetes Secret and mount it into the verifier pod.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Delivery row is `failed` with a transport error | The URL is not reachable **from the app process**. Use `host.docker.internal` or a cluster Service DNS, not `localhost` inside Jenkins. |
| Verifier returns 401 | Secret mismatch — a rotated secret invalidates the old one. Copy the current value from the **Rotate secret** response. |
| No delivery row at all | Webhook is inactive, or the event name is not subscribed. Check `GET /api/webhook-events` for exact names (`pr.opened`, not `pull_request`). |
| Jenkins build starts but checkout fails | `github-loco-pat` credential missing/wrong, or the branch was already deleted after merge. |
| Webhook retries cause duplicate builds | Use `X-GitHub-Delivery` as a dedupe key in the Jenkins job, and make the verifier return 2xx immediately. |
| PR event has no branch | PR payloads contain only `number`; resolve `head_ref` through `GET /api/repos/{owner}/{repo}/pulls/{number}` as in the Jenkinsfile. |

## Security notes

- The proxy is the only component that checks the HMAC. Without it, anyone who
  guesses the trigger token can start a build.
- Keep the webhook secret and the CI account password in Jenkins credentials /
  environment secrets; the plain values above are for local demo only.
- Do not publish Jenkins ports 8080/50000 outside your machine.
