$natsexe = Resolve-Path -Path $env:NATS_EXECUTABLE
$args = @("-a","127.0.0.1","-p","4222")

$ErrorActionPreference = "Stop"

$natstask = Start-Process -FilePath $natsexe -ArgumentList $args -NoNewWindow -PassThru


cargo test --verbose
$er = $LASTEXITCODE

$natstask.Kill()

exit $er