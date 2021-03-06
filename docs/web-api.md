# Web API

Web API endpoints listed below have been implemented at this moment:

| ENDPOINT                                        | COMPATIBLE WITH MIRAKURUN? |
|-------------------------------------------------|----------------------------|
| [/api/version]                                  |                            |
| [/api/status]                                   |                            |
| [/api/channels]                                 | :heavy_check_mark:         |
| [/api/channels/{channel_type}/{channel}/stream] | :heavy_check_mark:         |
| [/api/channels/{channel_type}/{channel}/services/{sid}/stream] |             |
| [/api/channels]                                 | :heavy_check_mark:         |
| [/api/services]                                 | :heavy_check_mark:         |
| [/api/services/{id}]                            | :heavy_check_mark:         |
| [/api/services/{id}/stream]                     | :heavy_check_mark:         |
| [/api/programs]                                 | :heavy_check_mark:         |
| [/api/programs/{id}]                            | :heavy_check_mark:         |
| [/api/programs/{id}/stream]                     | :heavy_check_mark:         |
| [/api/tuners]                                   | :heavy_check_mark:         |
| [/api/docs]                                     | :heavy_check_mark:         |
| [/api/iptv/playlist]                            |                            |

The endpoints above are enough to run [EPGStation].

It also enough to run [BonDriver_mirakc].  It's strongly recommended to
enable `SERVICE_SPLIT` in `BonDriver_mirakc.ini` in order to reduce network
traffic between mirakc and BonDriver_mirakc.  Because the
`/api/channels/{channel_type}/{channel}/stream` endpoint provides a **raw** TS
stream which means that all TS packets from a tuner will be sent even though
some of them don't need for playback.

Unfortunately, mirakc doesn't work with [BonDriver_Mirakurun] at this point due
to some kind of issue in BonDriver_Mirakurun or mirakc.
See [issues/4](https://github.com/masnagam/mirakc/issues/4) for details
(discussion in Japanese).

Web API endpoints listed below have been implemented as the mirakc extensions:

* [/api/iptv/playlist]

[/api/version]: #apiversion
[/api/status]: #apistatus
[/api/channels]: #apichannels
[/api/channels/{channel_type}/{channel}/stream]: #apichannelschannel_typechannelstream
[/api/channels/{channel_type}/{channel}/services/{sid}/stream]: #apichannelschannel_typechannelservicessidstream
[/api/services]: #apiservices
[/api/services/{id}]: #apiservicesid
[/api/services/{id}/stream]: #apiservicesidstream
[/api/programs]: #apiprograms
[/api/programs/{id}]: #apiprogramsid
[/api/programs/{id}/stream]: #apiprogramsidstream
[/api/tuners]: #apituners
[/api/docs]: #apidocs
[/api/iptv/playlist]: #apiiptvplaylist

## Incompatibility of the `X-Mirakurun-Priority` header

There are the following differences of the `X-Mirakurun-Priority` header between
mirakc and Mirakurun:

* Treated as 0 when the minimum value of `X-Mirakurun-Priority` headers is less
  than 0
* Treaded as 128 when the maximum value of `X-Mirakurun-Priority` headers is
  greater than 0
* Can grab a tuner which is used by other users regardless of their priorities
  if the priority is 128

## /api/version

Returns the version string.

## /api/status

Returns an empty object.

## /api/channels

Returns a list of channels.

Query parameters have **NOT** been supported.

## /api/channels/{channel_type}/{channel}/stream

Starts streaming for a channel.

## /api/channels/{channel_type}/{channel}/services/{sid}/stream

Starts streaming for a service in a channel.

Unlike Mirakurun, the `sid` must be a service ID.  In Mirakurun, the `sid` is a
service ID or an ID of the `ServiceItem` class.

## /api/services

Returns a list of services.

Query parameters have **NOT** been supported.

## /api/services/{id}

Returns a service.

## /api/services/{id}/stream

Starts streaming for a service.

## /api/programs

Returns a list of programs.

Query parameters have **NOT** been supported.

## /api/programs/{id}

Returns a program.

## /api/programs/{id}/stream

Starts streaming for a program.

The streaming will starts when the program starts and stops when the program
ends.

## /api/tuners

Returns a list of tuners.

Query parameters have **NOT** been supported.

## /api/docs

Returns a Swagger JSON data extracted from a Mirakurun by using the following
command:

```shell
./scripts/mirakurun-openapi-json -c -w 10 $MIRAKURUN_VERSION | \
  ./scripts/fixup-openapi-json >/etc/mirakurun.openapi.json
```

See also [issues/13](https://github.com/masnagam/mirakc/issues/13).

## /api/iptv/playlist

Returns a M3U8 playlist includes all TV services.

The format of the M3U8 playlist is compatible with EPGStation.

[EPGStation]: https://github.com/l3tnun/EPGStation
[BonDriver_mirakc]: https://github.com/epgdatacapbon/BonDriver_mirakc
