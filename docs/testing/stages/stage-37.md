# Stage 37: Access groups

Suite [`stage37-e2e.mjs`](../../../e2e/stage37-e2e.mjs) · 28 checks

This device creates a group and restricts a value to it. A teammate written only from the format documentation (`access-mate.mjs`) requests access, reads the value once granted, creates a group of its own, and adds this device after being asked. Removing the teammate gives the group a new key.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).
- The Node teammate `access-mate.mjs`.
- Creates a device key in the Windows Credential Manager of the account that runs the tests.

## Flow

### 1. Setup

- the collection is shared

### 2. A group made here

- the Access section starts empty, without a device key yet
- Tab moves from the name to Create group
- the new group lists this device as its member
- making a group here is not announced as a grant
- the device key shows once a group exists

### 3. A value restricted to it travels encrypted

- the type reads Restricted: Prod after saving
- the repository holds the token only as ciphertext
- the repository moves to format 8 once it has a group

### 4. The teammate makes a group of its own and asks to join Prod

- the teammate's group shows with no access and a way to ask
- the request shows the fingerprint the teammate computed
- an unreadable value is named as restricted, not as a typo
- the locked value is shown read-only with the way in
- a value of an environment is locked as well

### 5. Granting the teammate lets it read the value

- the granted teammate opens the token with its own key
- the request is gone after granting

### 6. This device asks to join Ops and the teammate lets it in

- asking shows that a member has to grant it
- the request carries this device's key and fingerprint
- after the grant this device is a member of Ops
- the grant is announced with who gave it
- Open variables opens the collection variables and closes the notice
- the teammate's value opens here
- the Environments page opens the value after Sync now, without a restart
- the environment's value is the teammate's
- the request no longer reports the value as missing

### 7. Removing the teammate gives Prod a new key

- removing asks first and says what the removed member keeps
- the values to change at their source are listed
- the removed teammate is out and the token moved to the new key

---

Previous: [Git changes with a long history](stage-36.md) · [Stage list](../README.md#stage-list) · Next: [Launch screen](stage-38.md)
