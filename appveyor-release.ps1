cargo login $env:CARGO_TOKEN
$er = $LASTEXITCODE
if ($er -eq 0) {
    cargo package
    cargo publish
}
exit $er