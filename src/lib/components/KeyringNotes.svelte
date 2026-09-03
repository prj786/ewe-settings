<script>
  // The keyring playbook, shared by every sign-in card (mirrors the shell's
  // Welcome / account cards). Tokens live ONLY in the login keyring. PAM
  // creates it with the login password at the greeter; a keyring made with
  // any other password (typed into a prompt hidden behind the first-run
  // overlay) rejects the login password forever — the one repair is a fresh
  // one at the next login, which "Reset the keyring" arranges.
  export let state = "ok"; // ok | missing | locked | unavailable
  export let trouble = false; // locked, or the last error was keyring-*
  export let resetDone = false;
  export let busy = false; // a sign-in is in flight: no repair offers mid-flight
  export let online = true;
  export let disabled = false;
  export let onReset = () => {};
  export let onLogOut = () => {};
</script>

{#if online && state === "locked" && !resetDone}
  <div class="px-4 py-2.5 text-xs text-dim">
    Your keyring is locked: a small “Unlock keyring” prompt will appear during sign-in — answer it
    with your login password.
  </div>
{:else if online && state === "missing" && !resetDone}
  <div class="px-4 py-2.5 text-xs text-dim">
    A small “Choose password for new keyring” prompt will appear during sign-in — use your login
    password so it unlocks by itself at every login.
  </div>
{:else if state === "unavailable"}
  <div class="px-4 py-2.5 text-xs text-warning">
    No Secret Service keyring is running — gnome-keyring must be installed and started for this
    session before sign-in can store its token.
  </div>
{/if}

{#if resetDone}
  <div class="flex items-center justify-between gap-3 px-4 py-3">
    <div class="text-xs text-dim">
      Keyring reset — log out and back in (it is recreated with your login password), then sign in
      again.
    </div>
    <button class="btn-ghost !py-1 text-xs" {disabled} on:click={onLogOut}>Log out now</button>
  </div>
{:else if trouble && !busy}
  <div class="flex items-center justify-between gap-3 px-4 py-3">
    <div class="text-xs text-dim">
      {state === "locked"
        ? "The keyring is locked and PAM could not unlock it with your login password."
        : "The keyring refused to store the token."}
      Replacing it makes a fresh keyring at the next login; the old files are kept in
      ~/.local/share/keyrings.bak.
    </div>
    <button class="btn-ghost !py-1 text-xs" {disabled} on:click={onReset}>Reset the keyring</button>
  </div>
{/if}
