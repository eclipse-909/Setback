# keepalive
This program utilizes 30% of the CPU for 5 minutes every 3 days to keep the server alive.

## Installing on the VPS
TODO settle on path to keepalive binary and update .crontab

## Oracle's Reclamation Policy
Reclamation of Idle Compute Instances

Idle Always Free compute instances may be reclaimed by Oracle. Oracle will deem virtual machine and bare metal compute instances as idle if, during a 7-day period, the following are true:
* CPU utilization for the 95th percentile is less than 20%
* Network utilization is less than 20%
* Memory utilization is less than 20% (applies to A1 shapes only)