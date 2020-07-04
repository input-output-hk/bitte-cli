require "./bitte/secrets"

if ARGV.empty?
  secrets = Bitte::SecretService.new

  # At this point we don't have the users/groups setup yet, but this avoids
  # having more than two phases.
  secrets.sync_secrets({mode: "0644", chown: "root:root"})
  secrets.sync_secrets
end
