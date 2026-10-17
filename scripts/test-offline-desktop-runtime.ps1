# Disposable Windows CI. Real installed Tauri app; NOT signed/user acceptance.
param(
 [Parameter(Mandatory=$true)][string]$ExecutablePath,
 [Parameter(Mandatory=$true)][string]$EvidencePath,
 [Parameter(Mandatory=$true)][string]$SourceRevision,
 [switch]$VerifyStoredStateOnly,
 [ValidateSet('same-version-repair','rejected-upgrade-preflight')][string]$StoredStateContext='same-version-repair'
)
$ErrorActionPreference='Stop'
$checks=[System.Collections.Generic.List[object]]::new()
$rules=[System.Collections.Generic.List[string]]::new()
$app=$null
$success=$false
$scope='NOT_VERIFIED'
function Record($key,$status,$note){
 $checks.Add([pscustomobject]@{check=$key;status=$status;detail=$note})
 Write-Host "$status $key : $note"
}
function Find-UI($root,[string]$name,[int]$seconds=25){
 $condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$name)
 $deadline=[datetime]::UtcNow.AddSeconds($seconds)
 do{
  $e=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
  if($null -ne $e){return $e}
  Start-Sleep -Milliseconds 500
 }while([datetime]::UtcNow -lt $deadline)
 if($name -eq 'Exactly where the preview came from'){
  $markers=@()
  foreach($signal in @('Scanning explicit temp and cache roots…','Smart Clean preview could not complete','Cancel scan','Scan cleanup candidates','Exactly where the preview came from')){
   try{
    $c=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$signal)
    if($null -ne $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$c)){$markers+=$signal}
   }catch{$markers+='UIA diagnostic unavailable'}
  }
  throw "Preview total not observed in $seconds seconds; UI markers: $($markers -join '; ')"
 }
 throw "Missing actual UIA element: $name"
}
function Click-UI($root,[string]$name){
 $element=Find-UI $root $name
 if(-not $element.Current.IsEnabled){throw "UIA control is disabled: $name"}
 $pattern=$null
 if($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$pattern)){
  ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
  return
 }
 # ARIA-pressed buttons can expose TogglePattern instead of InvokePattern.
 # Permit only the exact, non-destructive preference controls.
 $preferenceNames=@('Enable central management','Dark theme not selected','Compact sidebar off')
 if($name -notin $preferenceNames -or
    $element.Current.ControlType -ne [System.Windows.Automation.ControlType]::Button){
  throw "Unsupported UIA InvokePattern for '$name' (control type: $($element.Current.ControlType.ProgrammaticName))."
 }
 $pattern=$null
 if($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern,[ref]$pattern)){
  ([System.Windows.Automation.TogglePattern]$pattern).Toggle()
  return
 }
 # If Edge/WebView2 exposes neither action pattern, use the real keyboard
 # activation of this focused button. Fail closed unless UIA confirms that
 # the exact target button, not a neighboring control, holds keyboard focus.
 if(-not $element.Current.IsKeyboardFocusable){
  throw "Preference control has no UIA Invoke/TogglePattern and cannot receive keyboard focus."
 }
 $element.SetFocus()
 $focused=[System.Windows.Automation.AutomationElement]::FocusedElement
 if($null -eq $focused -or $focused.Current.Name -ne $name -or
    $focused.Current.ControlType -ne [System.Windows.Automation.ControlType]::Button -or
    ([string]::Join('.', $focused.GetRuntimeId())) -ne ([string]::Join('.', $element.GetRuntimeId()))){
  throw "Preference control focus could not be verified; refusing blind keyboard input."
 }
 Add-Type -AssemblyName System.Windows.Forms
 [System.Windows.Forms.SendKeys]::SendWait('{SPACE}')
}
function Open-App{
 $script:app=Start-Process -FilePath $ExecutablePath -PassThru
 $deadline=[datetime]::UtcNow.AddSeconds(40)
 do{
  $p=Get-Process -Id $script:app.Id -ErrorAction SilentlyContinue
  if($null -eq $p){throw 'Native application exited early.'}
  $p.Refresh()
  if($p.MainWindowHandle -ne [intptr]::Zero){
   return [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
  }
  Start-Sleep -Milliseconds 500
 }while([datetime]::UtcNow -lt $deadline)
 throw 'Native main window not exposed in runner desktop session.'
}
function Close-App{
 if($null -eq $script:app){return}
 $p=Get-Process -Id $script:app.Id -ErrorAction SilentlyContinue
 if($null -ne $p){
  if(-not $p.CloseMainWindow()){$p.Kill()}
  elseif(-not $p.WaitForExit(10000)){$p.Kill()}
 }
 $script:app=$null
}
try{
 if(-not(Test-Path -LiteralPath $ExecutablePath -PathType Leaf)){throw 'Installed executable missing.'}
 Add-Type -AssemblyName UIAutomationClient
 Add-Type -AssemblyName UIAutomationTypes
 Record 'package' 'PASS' 'Installed native executable exists.'
 $paths=[System.Collections.Generic.List[string]]::new()
 $paths.Add((Resolve-Path -LiteralPath $ExecutablePath).Path)
 $roots=@()
 $pf86=[Environment]::GetFolderPath('ProgramFilesX86')
 if($pf86){$roots+=Join-Path $pf86 'Microsoft\EdgeWebView\Application'}
 if($env:LOCALAPPDATA){$roots+=Join-Path $env:LOCALAPPDATA 'Microsoft\EdgeWebView\Application'}
 foreach($root in $roots){
  if(Test-Path -LiteralPath $root){
   Get-ChildItem -LiteralPath $root -Recurse -File -Filter msedgewebview2.exe -ErrorAction Stop |
    ForEach-Object {if(-not $paths.Contains($_.FullName)){$paths.Add($_.FullName)}}
  }
 }
 if($paths.Count -lt 2){throw 'WebView2 binary unavailable for scoped network block.'}
 $i=0
 foreach($path in $paths){
  $ruleName="PCManagerCI_Offline_$($PID)_$i"
  $null=New-NetFirewallRule -Name $ruleName -DisplayName $ruleName -Direction Outbound -Action Block -Program $path -Profile Any -Enabled True -ErrorAction Stop
  $rules.Add($ruleName)
  $rule=Get-NetFirewallRule -Name $ruleName -ErrorAction Stop
  $filter=$rule | Get-NetFirewallApplicationFilter
  if($rule.Action -ne 'Block' -or $rule.Enabled -ne 'True' -or $filter.Program -ne $path){throw 'Firewall rule readback mismatch.'}
  $i++
 }
 $scope='APP_AND_WEBVIEW2_OUTBOUND_BLOCKED'
 Record 'network' 'PASS' "Outbound denied for $($paths.Count) binaries, not whole-machine airgap."
 if($VerifyStoredStateOnly){
  # This is a second *real* installed launch after same-version NSIS repair.
  # Require state created in the earlier native smoke; no browser mock.
  $ui=Open-App
  if($null -eq $ui){throw 'No UI Automation root after repair reinstall.'}
  Click-UI $ui 'Settings'
  $null=Find-UI $ui 'Off: standalone mode; no automatic gateway sync.'
  $null=Find-UI $ui 'Dark theme selected'
  $null=Find-UI $ui 'Compact sidebar on'
  Click-UI $ui 'Health Check'
  $null=Find-UI $ui 'Latest completed scan' 30
  if($StoredStateContext -eq 'rejected-upgrade-preflight'){
   Record 'post-preflight-native-state' 'PASS' 'Rejected unsigned candidates left real WebView2 managed OFF, appearance and Health report intact.'
  }else{
   Record 'repair-state' 'PASS' 'Same-version unsigned NSIS repair retained managed OFF, appearance and Health report in real WebView2.'
  }
 }else{
 $ui=Open-App
 if($null -eq $ui){throw 'No UI Automation root.'}
 $null=Find-UI $ui 'Health Check V1 is ready' 30
 $null=Find-UI $ui 'Native bridge connected' 30
 $null=Find-UI $ui 'Standalone · Offline' 15
 Record 'startup' 'PASS' 'Native UI and bridge available with managed OFF.'
 Click-UI $ui 'Open Health Check'
 Click-UI $ui 'Scan now'
 $null=Find-UI $ui 'Latest completed scan' 90
 $null=Find-UI $ui 'Evidence by category'
 Record 'health' 'PASS' 'Actual Windows read-only scan completed.'
 Click-UI $ui 'Smart Clean'
 Click-UI $ui 'Scan cleanup candidates'
 # UI Automation may coalesce the non-interactive eyebrow "Preview total"
 # into its parent. Both following controls are rendered only when the actual
 # native scan has returned a completed CleanupScanSummary (not during scan).
 $null=Find-UI $ui 'Exactly where the preview came from' 120
 $null=Find-UI $ui 'Scan again' 15
 Record 'clean' 'PASS' 'Native scan returned a complete provider preview; no Delete invoked.'
 Click-UI $ui 'Settings'
 $null=Find-UI $ui 'Off: standalone mode; no automatic gateway sync.'
 Click-UI $ui 'Dark theme not selected'
 $null=Find-UI $ui 'Dark theme selected'
 Click-UI $ui 'Compact sidebar off'
 $null=Find-UI $ui 'Compact sidebar on'
 Record 'appearance' 'PASS' 'Dark theme and compact sidebar changed using real UI controls.'
 Click-UI $ui 'Enable central management'
 $null=Find-UI $ui 'On: optional signed gateway sync is enabled.'
 Record 'managed-on' 'PASS' 'Explicit managed ON observed with outbound rules still active.'
 Close-App
 $ui=Open-App
 Click-UI $ui 'Settings'
 $null=Find-UI $ui 'Dark theme selected'
 $null=Find-UI $ui 'Compact sidebar on'
 $null=Find-UI $ui 'On: optional signed gateway sync is enabled.'
 Click-UI $ui 'Health Check'
 $null=Find-UI $ui 'Latest completed scan' 30
 Record 'restart-on' 'PASS' 'Actual WebView2 restart restored ON, theme, sidebar and Health report.'
 Click-UI $ui 'Settings'
 Click-UI $ui 'Enable central management'
 $null=Find-UI $ui 'Off: standalone mode; no automatic gateway sync.'
 Close-App
 $ui=Open-App
 # Compact sidebar hides its badge from the UIA tree. Verify the persisted
 # OFF value through the visible Settings view rather than the hidden badge.
 Click-UI $ui 'Settings'
 $null=Find-UI $ui 'Off: standalone mode; no automatic gateway sync.'
 $null=Find-UI $ui 'Dark theme selected'
 $null=Find-UI $ui 'Compact sidebar on'
 Record 'restart-off' 'PASS' 'Explicit OFF and local appearance persisted after second process restart.'
 }
 $success=$true
}catch{
 Record 'runtime' 'FAIL' $_.Exception.Message
 throw
}finally{
 Close-App
 foreach($ruleName in $rules){Remove-NetFirewallRule -Name $ruleName -ErrorAction SilentlyContinue}
 $parent=Split-Path -Parent $EvidencePath
 if($parent){New-Item -ItemType Directory -Force -Path $parent | Out-Null}
 [pscustomobject]@{
  schema='pc-manager.offline-runtime-evidence/v1'
  sourceRevision=$SourceRevision
  checkoutRevision=(git rev-parse HEAD)
  kind='native-ui-automation-not-human-acceptance'
  verificationMode=$(if($VerifyStoredStateOnly){if($StoredStateContext -eq 'rejected-upgrade-preflight'){'rejected-upgrade-preflight'}else{'unsigned-same-version-repair'}}else{'native-offline-smoke'})
  networkScope=$scope
  productionSigned=$false
  userWindowsAccepted=$false
  success=$success
  checks=@($checks.ToArray())
  completedAtUtc=[datetime]::UtcNow.ToString('o')
 } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
}
