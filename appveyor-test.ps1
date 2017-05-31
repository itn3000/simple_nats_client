$natsexe = Resolve-Path -Path $env:NATS_EXECUTABLE
Get-ChildItem
$args = @("-a","127.0.0.1","-p","4222")

$ErrorActionPreference = "Stop"

$natstask = Start-Process -FilePath $natsexe -ArgumentList $args -NoNewWindow -PassThru

cargo test --verbose

$natstask.Kill()

