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
  pacman -Sy --noconfirm git pacman-contrib base-devel
  # To access aur via ssh aur public key must be added to known hosts
  mkdir -p ~/.ssh
  echo "aur.archlinux.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEuBKrPzbawxA/k2g6NcyV5jmqwJ2s+zpgZGZ7tpLIcN" >> ~/.ssh/known_hosts
  chmod 644 ~/.ssh/known_hosts

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
