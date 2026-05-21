# recu demo — render with: vhs docs/demo.tape
# Requires the `recu` binary on PATH (cargo build, then target/debug/recu).

Output docs/demo.gif

Set Shell zsh
Set FontSize 16
Set Width 1000
Set Height 700
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms
Set PlaybackSpeed 1.0

# Work on a throwaway copy of the sample data so re-renders stay reproducible
# and the committed examples/recu.csv is never mutated.
Hide
Type "cp examples/recu.csv /tmp/recu-demo.csv && export RECU_FILE=/tmp/recu-demo.csv && clear"
Enter
Show

# The headline view: amounts, due dates, category breakdown.
Type "recu"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Enter
Sleep 300ms

# Charges laid out on a month grid.
Type "recu calendar"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Enter
Sleep 300ms

# Add a new expense — gaming was the missing category.
Type "recu add 'PS Squared' -a 9.99 -c usd -i monthly --category Gaming"
Sleep 500ms
Enter
Sleep 2s

# Edit it interactively: pick a field, set a value, finish with Done.
Type "recu edit 'PS Squared'"
Sleep 500ms
Enter
Sleep 2s
Enter
Sleep 1500ms
Type "12.99"
Sleep 800ms
Enter
Sleep 1500ms
# Finish by selecting "Done" (last menu item) instead of cancelling.
Down@200ms 6
Sleep 500ms
Enter
Sleep 1.5s

# End a subscription non-interactively by @id — set its end date in the past.
Type "recu edit @1 --end 2026-05-01"
Sleep 500ms
Enter
Sleep 2s

Type "clear"
Enter
Sleep 300ms

# Updated list — PS Squared shows up, the ended @1 is gone.
Type "recu"
Sleep 500ms
Enter
Sleep 4s
