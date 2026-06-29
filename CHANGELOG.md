# Changelog

## [0.7.0](https://github.com/THernandez03/d/compare/v0.6.0...v0.7.0) (2026-06-29)


### Features

* ✨ Gold-colored version manager and program names in output ([bdd7aea](https://github.com/THernandez03/d/commit/bdd7aea06cec1e5f4539d78ef119f5f98ea2ccee))

## [0.6.0](https://github.com/THernandez03/d/compare/v0.5.1...v0.6.0) (2026-05-24)


### Features

* ✨ Colored help, -H/-v aliases, styled info/uninstall ([639bef8](https://github.com/THernandez03/d/commit/639bef8a4b7340f2349c0be503cf9ce7b822e18d))
* add binary releases and install.sh ([e1afc3f](https://github.com/THernandez03/d/commit/e1afc3f8b0246642cf71106c3effe41cf8f8352e))
* colorized install messages; fix uninstall() return type ([41ec267](https://github.com/THernandez03/d/commit/41ec26749b80f1fe1eb3233e0c82e8d6ba9f727e))
* display from/to version during activation ([a929fc6](https://github.com/THernandez03/d/commit/a929fc6d80290b2700a39613d7562fa7eb3aaf98))
* initial Deno version manager implementation ([bb61a9b](https://github.com/THernandez03/d/commit/bb61a9bc7180b72535257e2fb71e0e9f5b0985e2))
* restructure CLI, add Makefile, update README ([7c53ef9](https://github.com/THernandez03/d/commit/7c53ef9d0ef5ec79ea3c755f66e6063f814b0ee5))
* skip activation when version is already active ([92d43fc](https://github.com/THernandez03/d/commit/92d43fc677a216fdb65a7bc1c552b463dbe3184d))
* use dl.deno.land URLs and add nightly/edge aliases to canary channel ([cd130fa](https://github.com/THernandez03/d/commit/cd130fa9d196712866cf2528d9463dd2a1678092))


### Bug Fixes

* 🐛 Strip name prefix from self-update version tag ([5306d67](https://github.com/THernandez03/d/commit/5306d677a90955a27015e2a7709316ccf1a57ae0))
* remove stale uninstall tests, fix needless borrow in install.rs ([268493f](https://github.com/THernandez03/d/commit/268493f2a8a0af34acf28304e3245fde6053306b))
* resolve canary to canary-{sha} for stable cache key ([edaa4fe](https://github.com/THernandez03/d/commit/edaa4fe2bfc143524daf87a724705b6b1618bdfd))
* run tests single-threaded to avoid env-var data race between modules ([db721da](https://github.com/THernandez03/d/commit/db721daf34d62eaa954175c1d8178b858263fdcc))


### Documentation

* 📝 Document prune --force and uninstall --yes flags ([0bed769](https://github.com/THernandez03/d/commit/0bed76978215005e52481503feee1ec185bb8694))
* 📝 Document zsh 'd' alias conflict and unalias d fix ([3f62ba1](https://github.com/THernandez03/d/commit/3f62ba1a25a3f296f2ff1b0553cb08a505e1a8ea))
* add related projects section ([365db4f](https://github.com/THernandez03/d/commit/365db4ffa246c34efa3e2df6d32f4552069bf45f))

## [0.5.1](https://github.com/THernandez03/d/compare/d-v0.5.0...d-v0.5.1) (2026-05-24)


### Bug Fixes

* 🐛 Strip name prefix from self-update version tag ([5306d67](https://github.com/THernandez03/d/commit/5306d677a90955a27015e2a7709316ccf1a57ae0))

## [0.5.0](https://github.com/THernandez03/d/compare/d-v0.4.0...d-v0.5.0) (2026-05-24)


### Features

* ✨ Colored help, -H/-v aliases, styled info/uninstall ([639bef8](https://github.com/THernandez03/d/commit/639bef8a4b7340f2349c0be503cf9ce7b822e18d))
* add binary releases and install.sh ([e1afc3f](https://github.com/THernandez03/d/commit/e1afc3f8b0246642cf71106c3effe41cf8f8352e))
* colorized install messages; fix uninstall() return type ([41ec267](https://github.com/THernandez03/d/commit/41ec26749b80f1fe1eb3233e0c82e8d6ba9f727e))
* display from/to version during activation ([a929fc6](https://github.com/THernandez03/d/commit/a929fc6d80290b2700a39613d7562fa7eb3aaf98))
* initial Deno version manager implementation ([bb61a9b](https://github.com/THernandez03/d/commit/bb61a9bc7180b72535257e2fb71e0e9f5b0985e2))
* restructure CLI, add Makefile, update README ([7c53ef9](https://github.com/THernandez03/d/commit/7c53ef9d0ef5ec79ea3c755f66e6063f814b0ee5))
* skip activation when version is already active ([92d43fc](https://github.com/THernandez03/d/commit/92d43fc677a216fdb65a7bc1c552b463dbe3184d))
* use dl.deno.land URLs and add nightly/edge aliases to canary channel ([cd130fa](https://github.com/THernandez03/d/commit/cd130fa9d196712866cf2528d9463dd2a1678092))


### Bug Fixes

* remove stale uninstall tests, fix needless borrow in install.rs ([268493f](https://github.com/THernandez03/d/commit/268493f2a8a0af34acf28304e3245fde6053306b))
* resolve canary to canary-{sha} for stable cache key ([edaa4fe](https://github.com/THernandez03/d/commit/edaa4fe2bfc143524daf87a724705b6b1618bdfd))
* run tests single-threaded to avoid env-var data race between modules ([db721da](https://github.com/THernandez03/d/commit/db721daf34d62eaa954175c1d8178b858263fdcc))


### Documentation

* 📝 Document prune --force and uninstall --yes flags ([0bed769](https://github.com/THernandez03/d/commit/0bed76978215005e52481503feee1ec185bb8694))
* 📝 Document zsh 'd' alias conflict and unalias d fix ([3f62ba1](https://github.com/THernandez03/d/commit/3f62ba1a25a3f296f2ff1b0553cb08a505e1a8ea))
* add related projects section ([365db4f](https://github.com/THernandez03/d/commit/365db4ffa246c34efa3e2df6d32f4552069bf45f))

## [0.4.0](https://github.com/THernandez03/d/compare/v0.3.1...v0.4.0) (2026-05-24)


### Features

* ✨ Add --force to prune and --yes/-y to uninstall ([7089287](https://github.com/THernandez03/d/commit/70892873571d4a9adc10d6b884a237fc4f8bdee0))
