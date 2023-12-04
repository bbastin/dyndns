# DynDNS

This is a simple DynDNS server. If you run this server, it allows you to update DNS records with your dynamic
IP address that is assigned by your Internet Service Provider (ISP).

For now, this server only connects to Hetzner's DNS API. It is tested with AVM's Fritzbox router,
though any router that allows DynDNS with a flexible enough configuration format should work.

## Disclaimer

This is a small project that arose from my personal need for DynDNS, which sadly Hetzner does not provide natively, though their API and documentation allowed for a rather easy solution.
Still, this project is not thoroughly vetted. I am no Rust expert, but of course I tried my best.

This server is not stable right now. Almost everything that can fail will lead to a panic, as the error handling is fairly primitive. Still, it works, at least for me, and errors that I encounter will probably be fixed rather shortly, at least if I have the time.

You can find all config files from within this **README.md** file in the **example-config** folder.

## Configuring your Fritzbox

If you want to enable your Fritzbox to use your DynDNS server,
open your settings (by default, they are available under http://fritz.box or http://192.168.178.1)
and open **Internet -> Shares -> DynDNS**.
After checking the checkbox and choosing **Other Provider**,
you can enter the update URL

    https://[YOUR SERVER ADDRESS]/update?user=<username>&password=<pass>&host=<domain>&ip=<ipaddr>&ip6=<ip6addr>

, where you replace *[YOUR SERVER ADDRESS]* with the address your server will be avaible under.
Afterwards, you can set up a username, password and the domain you want to update.
Those entries have to match the entries in your **config.json**.

## Getting your Hetzner API key

Open your DNS console under https://dns.hetzner.com. In the upper righthand corner,
you can find your profile information.
Click on **API tokens** and create a new API token.
You have to only copy that token into the **config.json** file.
For this example, let's assume our API token is ```aiodQ83HFSDIj3iAHfOIAIAOWUR```

You also have to make sure that the record you want to update already exists!
Select the appropiate zone and create an A record with your current IPv4 address and/or
an AAAA record with your current IPv6 address.
You can either use the domain itself or a subdomain.

## Creating your **config.json**

**config.json** contains all the information required for the DynDNS server to validate requests.
You can set your username and password yourself;
those are only required to assure that you do not accept any IP address.
Within the domains section, you can configure domains that you want to update.

With the HetznerProvider, you set the value of *provider* to "HetznerProvider",
then enter your API token from the step before as *apitoken*.
The *host* key holds the domain you want to update.
Within the zone entry, you can enter the zone where the domain resides.
If *host* is a subdomain, this will be the top most parent domain you control.
If *host* is the domain itself, just enter the domain itself.
The *id* has to be set to the ID of the zone;
sadly, this is not being displayed within the DNS console,
but you can use the API to find it:

    curl https://dns.hetzner.com/api/v1/zones -H "Auth-API-Token: aiodQ83HFSDIj3iAHfOIAIAOWUR" | json_pp

Below you can find an example **config.json**.

    {
        "name": "exampleuser",
        "password": "exampleuserpassword",
        "domains": [{
            "provider": "HetznerProvider",
            "apitoken": "aiodQ83HFSDIj3iAHfOIAIAOWUR",
            "host": "test.example.com",
            "zone": {
                "name": "example.com",
                "id": "notused"
            }
        }]
    }

## Building and starting the server

Building and starting the server is fairly easy as you only have to run ```cargo run```.

If you want to generate a binary you can upload to a server which does not have a Rust toolchain,
you can run ```cargo build --release```. This will generate the executable **target/release/dyndns**.
You can simply upload this binary to a server an run it there.

## Running behind a reverse proxy like nginx

For security reasons, it is **not recommended** to run this server without TLS encryption/HTTPS,
as this would transmit your login information in plain text.
An easy way to enable encryption is to use this server behind a reverse proxy like nginx.

In order to prevent this server to just bind to all addresses and directly accept requests from outside,
create a file called **Rocket.toml** in the directory of the server.

    [default]
    address = "127.0.0.1"
    port = 8079
    workers = 1

An example nginx configuration could look something like the following. Please note the *<domainname>* entries that should be changed to the domain that the server will be available under. Also note the port number in the upstream section that matches the port number in the **Rocket.toml** file.

    upstream dyndns {
        server localhost:8079;
    }

    server {
            listen 80;
            listen [::]:80;

            server_name <domainname>;

            root /srv/http/<domainname>/;
            index index.xhtml index.html;

            location /.well-known/acme-challenge {}

            location / {
                    return 308 https://$host$request_uri;
            }
    }

    server {
            listen 443 ssl http2;
            listen [::]:443 ssl http2;

            ssl_certificate         /etc/letsencrypt/live/<domainname>/fullchain.pem;
            ssl_certificate_key     /etc/letsencrypt/live/<domainname>/privkey.pem;

            server_name <domainname>;

            root /srv/http/<domainname>/;
            index index.xhtml index.html;

            location / {
                    proxy_pass http://dyndns;
            }
    }

Please consult the nginx documentation as well as Let's Encrypt for more information;
this is only a hint and a start, but no full explanation.

## Integrating with systemd

Usually, you would want to integrate the server with systemd
in order to keep it running and start it automatically on boot.
Below you can find an example service unit which you can install
on your system at **/etc/systemd/system/dyndns.service**.

This tutorial assumes you are running both systemd and journald. For other init and/or logging services, you have to consult their respective documentations.
Please consult the systemd documentation as well as for more information;
this is only a hint and a start, but no full explanation.

    [Unit]
    Description=DynDNS server
    After=syslog.target
    After=network.target

    [Service]
    Type=simple
    User=www-data
    Group=www-data
    WorkingDirectory=/srv/dyndns
    ExecStart=/srv/dyndns/dyndns

    [Install]
    WantedBy=multi-user.target

After installing the file
(and making sure that both *WorkingDirectory* and *ExecStart* point to the correct location)
you then have to make sure systemd knows about this unit by running

    systemctl daemon-reload

. Afterwards, you can start the server with

    systemctl start dyndns

and stop it with

    systemctl stop dyndns

. If you want the service to be started automatically on boot, you can enable it using

    systemctl enable dyndns

. If you need to access the logs, you can use journald:

    journalctl -u dyndns
