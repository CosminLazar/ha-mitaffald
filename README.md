# ha-mitaffald
[![codecov](https://codecov.io/gh/CosminLazar/ha-mitaffald/graph/badge.svg?token=09VSTRLEGG)](https://codecov.io/gh/CosminLazar/ha-mitaffald)

Fetches the garbage container pick-up schedule from Kredsløb for the specified address and publishes the information to [HomeAssistant](https://github.com/home-assistant).

The address can be specified as one of the following:
 - `TraditionalAddress` - by supplying `street_name`, `street_no`, `postal_code` and `city`
 - `AddressId` - the internal address id that Kredsløb uses - Fetching the Id can be done [here](https://www.kredslob.dk/privat/genbrug-og-affald/toemmekalender) by looking at the network requests in the browser.
