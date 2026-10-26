// @ts-check
import { defineConfig } from 'astro/config';

// Dev and preview stay on loopback. Port 4321 is the public-web contract.
// Astro moves to the next port if 4321 is already taken; local-run.md records that.
export default defineConfig({
  server: {
    host: '127.0.0.1',
    port: 4321,
  },
});
