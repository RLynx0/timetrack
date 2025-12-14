function __timetrack_activity_subcommands
    timetrack activity help | awk '
        /^\w/ { c = 0 }
        /^Commands:/ { c = 1; next }
        !/^$/ && c {
            com = $1
            gsub(/^\s*\w+\s*/, "", $0)
            printf "%s\t\'%s\'\n", com, $0
        }
    '
end
function __timetrack_subcommands
    timetrack help | awk '
        /^\w/ { c = 0 }
        /^Commands:/ { c = 1; next }
        !/^$/ && c {
            com = $1
            gsub(/^\s*\w+\s*/, "", $0)
            printf "%s\t\'%s\'\n", com, $0
        }
    '
end
function __timetrack_attendance_types
    timetrack list-attendance-types -m \
        | awk -F "\t" '{ print $1 "\t\'" $2 "\'" }'
end
function __timetrack_activities
    set parent (commandline -ct | string replace -r '[^/]+$' '')
    timetrack activity ls -m $parent | awk -v p="$parent" -F "\t" '
        { printf "%s", p }
        $2 { printf "%s\t\'%s\'\n", $1, $2; next }
        { print $1 }
    '
end

complete -c timetrack -f \
    -n __fish_use_subcommand \
    -a "(__timetrack_subcommands)"

complete -c timetrack -f \
    -n '__fish_seen_subcommand_from activity' \
    -a"(__timetrack_activity_subcommands)"

complete -c timetrack \
    -rfl attendance \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_attendance_types)"
complete -c timetrack \
    -rfs a \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_attendance_types)"
complete -c timetrack -f \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_activities)"
complete -c timetrack -fl verbose \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack -fs v \
    -n '__fish_seen_subcommand_from start'
