final: prev: {

  bitte = final.callPackage ./package.nix { };
  
  # TODO: remove bitteShellCompat after the change to numtide/devshell is complete
  terraform-with-plugins = null; # impl provided by iog/bitte repo
  scaler-guard = null;           # impl provided by iog/bitte repo
  bitteShellCompat = final.callPackage ./old-bitte-shell.nix { };

}
