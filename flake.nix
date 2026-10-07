{
  description = "A flake to build and push Docker containers to GHCR";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      lib = nixpkgs.lib;
      ignoredNames = [
        "scripts"
        "build"
        "dist"
        "node_modules"
        "result"
      ];

      candidateDirs = lib.filterAttrs (
        name: type:
        type == "directory"
        && !(lib.hasPrefix "." name)
        && !(lib.hasPrefix "_" name)
        && !(builtins.elem name ignoredNames)
      ) (builtins.readDir ./.);

      workshopDirs = lib.filterAttrs (
        name: _: builtins.pathExists (./. + "/${name}/workshop.toml")
      ) candidateDirs;

      workshops = lib.mapAttrs' (
        name: _:
        let
          data = builtins.fromTOML (builtins.readFile (./. + "/${name}/workshop.toml"));
        in
        lib.nameValuePair data.name data
      ) workshopDirs;

      workshopList = builtins.attrValues workshops;

      requiredModels = lib.naturalSort (lib.unique (lib.filter (m: m != null && m != "") (
        map (w: w.model or null) workshopList
      )));

      requiredImages = lib.naturalSort (lib.unique (lib.concatMap (
        w:
        (lib.optional (w ? user && w.user ? image && w.user.image != "") w.user.image)
        ++ (lib.optional (w ? service && w.service ? image && w.service.image != "") w.service.image)
      ) workshopList));

      workshopInfo = {
        inherit requiredModels requiredImages;
      };

      findContainers =
        dir:
        let
          root = lib.optional (builtins.pathExists (./. + "/${dir}/Dockerfile")) {
            name = dir;
            path = dir;
          };
          subDirs = lib.filterAttrs (
            subName: subType:
            subType == "directory"
            && !(lib.hasPrefix "." subName)
            && !(lib.hasPrefix "_" subName)
            && builtins.pathExists (./. + "/${dir}/${subName}/Dockerfile")
          ) (builtins.readDir (./. + "/${dir}"));
          subs = lib.mapAttrsToList (subName: _: {
            name = "${dir}-${subName}";
            path = "${dir}/${subName}";
          }) subDirs;
        in
        root ++ subs;

      containers = lib.sort (a: b: a.name < b.name) (
        lib.concatLists (map findContainers (builtins.attrNames candidateDirs))
      );

      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      inherit workshops workshopInfo containers;

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          mkBuildScript =
            c:
            pkgs.writeShellScriptBin "build-${c.name}" ''
              exec ${./scripts/build_container.sh} "${c.name}" "${c.path}" "$@"
            '';

          buildAll = pkgs.writeShellScriptBin "build-all-images" ''
            exec ${./scripts/build_container.sh} --all "" "$@"
          '';
        in
        (builtins.listToAttrs (map (c: lib.nameValuePair "build-${c.name}" (mkBuildScript c)) containers))
        // { build-all-images = buildAll; }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          pkgsForSystem = self.packages.${system};
        in
        {
          default = pkgs.mkShell {
            buildInputs =
              with pkgs;
              [
                docker
                curl
                jq
              ]
              ++ (builtins.attrValues pkgsForSystem);

            shellHook = ''
              echo "Workshop Container Build Shell Initialized"
              echo "------------------------------------------"

              if [[ -n "''${GHCR_PAT:-}" && -n "''${GITHUB_USERNAME:-}" ]]; then
                echo "Authenticating with GHCR as ''${GITHUB_USERNAME}..."
                echo "''${GHCR_PAT}" | docker login ghcr.io -u "''${GITHUB_USERNAME}" --password-stdin >/dev/null 2>&1
                echo "  GHCR authentication successful."
              elif [[ -n "''${GHCR_PAT:-}" ]]; then
                echo "Authenticating with GHCR..."
                echo "''${GHCR_PAT}" | docker login ghcr.io -u "USER" --password-stdin >/dev/null 2>&1
                echo "  GHCR authentication successful."
              fi

              echo "------------------------------------------"
              echo "Available build commands:"
              echo "  build-all-images [--push] [--tag <tag>]"
              ${builtins.concatStringsSep "\n" (map (c: "echo \"  build-${c.name} [--push] [--tag <tag>]\"") containers)}
            '';
          };
        }
      );
    };
}
