# Security policy

## Supported versions

Security fixes go into the latest release. Please update to it before reporting a problem.

## Reporting a vulnerability

Do not open a public issue for a security problem.

Report it privately through GitHub: open the **Security** tab of this repository and choose **Report a vulnerability**. Include:

- the version of Codestamp and of Windows,
- what an attacker could do, and what they would need (for example a crafted collection file, a malicious server, or a shared Git repository),
- the steps to reproduce it, with a sample file if one is needed.

You will get a reply in the private advisory. Once a fix is released, the advisory is published with credit to you, unless you prefer to stay anonymous.

## Scope

In scope are problems in Codestamp itself, for example:

- a file Codestamp imports or syncs (Postman, OpenAPI, WSDL, a Collaboration repository) that runs code, reads files it should not, or leaks secrets,
- secret values reaching logs, history, exports, Git or the screen without being asked for,
- the script sandbox reaching files, processes or the network beyond `pm.sendRequest`,
- TLS checks being skipped, or credentials being sent to another host.

Out of scope are problems in the servers you call with Codestamp, and attacks that need someone who already controls your Windows account.
