# Setback Daemon

## Files
/
├── etc/systemd/system
│	├── setback.path -> /var/lib/setback/latest/systemd/setback.path
│	├── setback.service -> /var/lib/setback/latest/systemd/setback.service
│	├── setback_website.path -> /var/lib/setback/latest/systemd/setback_website.path
│	└── setback_website.service -> /var/lib/setback/latest/systemd/setback_website.service
├── usr/local/bin
│	├── setbackd -> /var/lib/setback/latest/bin/setbackd
│	├── setback_service -> /var/lib/setback/latest/bin/setback_service
│	└── setback_website -> /var/lib/setback/latest/bin/setback_website
└── var/lib/setback
	├── latest -> /var/lib/setback/v0.2.0
	├── v0.1.0
	│	├── bin
	│	│	├── setbackd
	│	│	├── setback_service
	│	│	└── setback_website
	│	└── systemd
	│		├── setback.path
	│		├── setback.service
	│		├── setback_website.path
	│		└── setback_website.service
	├── v0.1.1
	│	└── ...
	├── v0.2.0
	│	└── ...
	├── v...
	│	└── ...
	├── stage
	│	└── setback.tar.gz
	└── pidfile