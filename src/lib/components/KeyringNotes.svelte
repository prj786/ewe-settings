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
  import Row from "./ui/Row.svelte";
</script>

{#if online && state === "locked" && !resetDone}
  <Row sub="Your keyring is locked. A small “Unlock keyring” prompt appears when you sign in; answer it with your sign-in password." />
{:else if online && state === "missing" && !resetDone}
  <Row sub="A small “Choose password for new keyring” prompt appears when you sign in. Use your sign-in password, so it unlocks by itself every time." />
{:else if state === "unavailable"}
  <Row><span class="text-warning">No Secret Service keyring is running. Install and start gnome-keyring for this session before signing in, so it can keep the token.</span></Row>
{/if}

{#if resetDone}
  <Row sub="Keyring reset. Sign out and back in (it's made again with your sign-in password), then sign in to the account again.">
    <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" {disabled} on:click={onLogOut}>Sign out now</button>
  </Row>
{:else if trouble && !busy}
  <Row
    sub={(state === "locked"
      ? "The keyring is locked and your sign-in password couldn't unlock it."
      : "The keyring refused to keep the token.") +
      " Replacing it makes a fresh keyring the next time you sign in; the old files stay in ~/.local/share/keyrings.bak."}
  >
    <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" {disabled} on:click={onReset}>Reset keyring</button>
  </Row>
{/if}
