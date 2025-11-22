#!/usr/bin/env -S just --justfile

test:
  cargo test --bins --lib && cargo test --doc

integration-test:
  cargo test --test live_integration_tests

doc PACKAGE:
  cargo +nightly rustdoc -p {{PACKAGE}} --all-features -- --cfg docsrs

release-youtui-to-aur NEW_YOUTUI_VERSION:
  #!/usr/bin/env bash
  # https://just.systems//man/en/safer-bash-shebang-recipes.html?highlight=euxo#safer-bash-shebang-recipes
  set -euxo pipefail
  echo "Attempting to update AUR"

  git clone ssh://aur@aur.archlinux.org/youtui.git youtui-aur
  cd youtui-aur
  sed -i "s/^pkgver=.*/pkgver={{NEW_YOUTUI_VERSION}}/" PKGBUILD
  sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD

  updpkgsums
  # Consider building - rust toolchain required though.
  # makepkg -f --cleanbuild --nodeps 
  makepkg --printsrcinfo > .SRCINFO
  git commit -am 'nick42d-bot: Updating to latest due to GitHub release'
  git push
  echo "AUR update succesful"
