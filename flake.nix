{
  description = "A flake to build and push Docker containers to GHCR";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      catalog = builtins.fromJSON (builtins.readFile ./catalog.json);
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      workshops = catalog.workshops;
      workshopInfo = catalog.workshopInfo;

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          mkBuildScript =
            c:
            pkgs.writeShellScriptBin "build-${c.name}" ''
              exec ${./scripts/build_container.sh} "${c.name}" "${c.path}" "$@"
            '';

          buildScripts = map mkBuildScript catalog.containers;

          buildAllScript = pkgs.writeShellScriptBin "build-all-images" ''
            exec ${./scripts/build_container.sh} --all "" "$@"
          '';

          allPackages = buildScripts ++ [ buildAllScript ];
        in
        builtins.listToAttrs (map (p: { name = p.name; value = p; }) allPackages)
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          pythonEnv = pkgs.python3.withPackages (ps: [ ps.pyyaml ]);
          compileCatalog = pkgs.writeShellScriptBin "compile-catalog" ''
            exec ${pythonEnv}/bin/python scripts/compile_catalog.py "$@"
          '';

          pkgsForSystem = self.packages.${system};
        in
        {
          default = pkgs.mkShell {
            buildInputs =
              with pkgs;
              [
                docker
                curl
                talosctl
                kubectl
                kubernetes-helm
                tilt
                openssl
                zsh
                k9s
                cilium-cli
                hubble
                pythonEnv
                compileCatalog
              ]
              ++ (builtins.attrValues pkgsForSystem);

            # ShellHook will now automatically use your .envrc variables to log in
            shellHook = ''
              export KUBECONFIG="$(pwd)/.talos/kubeconfig"
              echo "📦 Container Build Shell Initialized"
              echo "------------------------------------"

              # Auto-login to GHCR
              if [[ -n "''${GHCR_PAT:-}" && -n "''${GITHUB_USERNAME:-}" ]]; then
                echo "🔑 Authenticating with GHCR as ''${GITHUB_USERNAME}..."
                echo "''${GHCR_PAT}" | docker login ghcr.io -u "''${GITHUB_USERNAME}" --password-stdin >/dev/null 2>&1
                echo "   ✅ GHCR Login successful."
              else
                echo "   ⚠️ GHCR credentials missing in .envrc"
              fi

              # Auto-login to Docker Hub
              if [[ -n "''${DH_PAT:-}" && -n "''${DH_UNAME:-}" ]]; then
                echo "🔑 Authenticating with Docker Hub as ''${DH_UNAME}..."
                echo "''${DH_PAT}" | docker login -u "''${DH_UNAME}" --password-stdin >/dev/null 2>&1
                echo "   ✅ Docker Hub Login successful."
              else
                echo "   ⚠️ Docker Hub credentials missing in .envrc"
              fi
              echo "------------------------------------"
              echo "Available commands:"
              echo "  compile-catalog [--check]"
              echo "  build-all-images [--push] [--tag <tag>]"
              ${builtins.concatStringsSep "\n" (map (c: "echo \"  build-${c.name} [--push] [--tag <tag>]\"") catalog.containers)}
            '';
          };
        }
      );
    };
}
