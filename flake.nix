{
  description = "A flake to build and push Docker containers to GHCR";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          # --- Script Generation Logic ---
          mkContainerScripts =
            containers:
            let
              scriptPreamble = ''
                #!/usr/bin/env bash
                set -euo pipefail

                # Using ROOT_DIR from your .envrc
                if [[ -z "''${ROOT_DIR:-}" ]]; then
                  echo "Error: ROOT_DIR is not set. Is direnv active?"
                  exit 1
                fi
              '';

              mkOneScript =
                {
                  name,
                  path,
                  type ? "docker",
                }:
                let
                  scriptName = "upload-${name}";

                  # Shared logic to require a tag argument for individual scripts
                  tagHandling = ''
                    if [ $# -eq 0 ]; then
                      echo "Error: No tag provided."
                      echo "Usage: ${scriptName} <semver-tag> (e.g., ${scriptName} v0.1.2)"
                      exit 1
                    fi
                    TAG=$1
                  '';

                  dockerBody = ''
                    ${tagHandling}
                    echo "--- Processing image: ${name} (Tag: $TAG) ---"
                    LOCAL_TAG="${name}:$TAG"
                    REMOTE_TAG="ghcr.io/nbhdai/${name}:$TAG"
                    CONTEXT_PATH="$ROOT_DIR/${path}"

                    echo "Building $LOCAL_TAG from $CONTEXT_PATH..."
                    docker build --platform linux/amd64 -t "$LOCAL_TAG" "$CONTEXT_PATH"

                    echo "Tagging $LOCAL_TAG as $REMOTE_TAG..."
                    docker tag "$LOCAL_TAG" "$REMOTE_TAG"

                    echo "Pushing $REMOTE_TAG..."
                    docker push "$REMOTE_TAG"
                    echo "Successfully pushed $REMOTE_TAG"
                    echo "-----------------------------------"
                  '';

                  nixBody = ''
                    ${tagHandling}
                    echo "--- Processing image: ${name} (Tag: $TAG) ---"
                    LOCAL_TAG="${name}:$TAG"
                    REMOTE_TAG="ghcr.io/nbhdai/${name}:$TAG"
                    RESULT_LINK="result-${name}"

                    echo "Building Nix attribute: ${path}..."
                    nix build --platform linux/amd64 "$ROOT_DIR#${path}" --out-link "$RESULT_LINK"

                    echo "Loading $LOCAL_TAG into Docker..."
                    docker load < "$RESULT_LINK"

                    echo "Tagging $LOCAL_TAG as $REMOTE_TAG..."
                    docker tag "$LOCAL_TAG" "$REMOTE_TAG"

                    echo "Pushing $REMOTE_TAG..."
                    docker push "$REMOTE_TAG"

                    rm "$RESULT_LINK"
                    echo "Successfully pushed $REMOTE_TAG"
                    echo "-----------------------------------"
                  '';

                  scriptBody = if type == "docker" then dockerBody else nixBody;
                in
                pkgs.writeShellScriptBin scriptName (scriptPreamble + scriptBody);

              mkAllScript =
                containers:
                let
                  calls = map (c: ''
                    echo "Triggering upload-${c.name} with tag $TAG..."
                    upload-${c.name} "$TAG"
                  '') containers;
                in
                pkgs.writeShellScriptBin "upload-all-images" ''
                  ${scriptPreamble}

                  if [ $# -eq 0 ]; then
                    echo "Error: No tag provided."
                    echo "Usage: upload-all-images <semver-tag> (e.g., upload-all-images v0.1.2)"
                    exit 1
                  fi

                  TAG=$1

                  echo "=== 🚀 Starting upload for all images with tag: $TAG ==="
                  echo ""
                  ${builtins.concatStringsSep "\n" calls}
                  echo ""
                  echo "=== ✅ All images pushed successfully! ==="
                '';

            in
            (map mkOneScript containers) ++ [ (mkAllScript containers) ];

          myContainers = [
            {
              name = "yolo-l2-notebook";
              path = "yolo-l2/notebook";
            }
            {
              name = "yolo-l2-verification";
              path = "yolo-l2/verification";
            }
            {
              name = "email-indirect-service";
              path = "email-indirect/service";
            }
            {
              name = "email-indirect-user";
              path = "email-indirect/user";
            }
            {
              name = "rag-poisoning-user";
              path = "rag-poisoning/user";
            }
            {
              name = "llm-embeddings";
              path = "llm-embeddings";
            }
            {
              name = "rag-poisoning-service";
              path = "rag-poisoning/service";
            }
            {
              name = "prompt-extraction-service";
              path = "prompt-extraction/service";
            }
            {
              name = "prompt-extraction-user";
              path = "prompt-extraction/user";
            }
          ];

          myContainerScripts = mkContainerScripts myContainers;

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
              ]
              ++ myContainerScripts;

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
              echo "  upload-all-images <tag>"
              ${builtins.concatStringsSep "\n" (map (c: "echo \"  upload-${c.name} <tag>\"") myContainers)}
            '';
          };
        }
      );
    };
}
