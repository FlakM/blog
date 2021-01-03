---
title: "Watering pi"
date: 2021-01-02T14:13:56+02:00
draft: true
authors: ["Maciej Flak"]
images: [
    "/images/yannick-pipke-GtcA8mw0t1U-unsplash.jpg", 
    "/images/rustypi/watering/watering_bb.png"
    ]
featured_image: '/images/yannick-pipke-GtcA8mw0t1U-unsplash.jpg'

tags: ["rust", "raspberry pi"]

Summary: '
Creating plan watering automation using rust 🦀 and raspberry pi
'

series: ["raspberry pi"]
---

{{< rusty-github >}}

More then year ago I picked up the idea of using new found love rust programming language to learn about electronics.
For additional wife's approval I decided to build plant watering automation.
So I sketched some basic requirements:

1. I want it to be autonomous apart from refilling water container (it should remind me of that though)
2. I want it to be smart - I don't wanna kill my fellow plants
3. I want it to have some storage capacity - don't we all love some cool graphs?
4. Cloud is ok but self hosted is better

Since I have never had any formal training in electronics i picked up some books and came up with some smaller projects on the way (blinking led, DHT11 sensor, ADC and some fun one off pocs).
Once I gained a little bit more confidence I started working on final project.

I decided on raspberry pi as the base for my project, mainly because I've already had one but also when doing my research I've came to conclusion that popular wifi capable alternatives (mainly ESP8266 and ESP32) are more cumbersome to work with with rust ([work is being done though](https://github.com/espressif/llvm-project/issues/4) and also RISCV version as far as I understand [RISC V](https://www.espressif.com/en/news/ESP32_C3) should be usefull too). Other alternative that I think would work was definitely [Arduino Uno](https://store.arduino.cc/arduino-uno-rev3).
Raspberry pi gave some awesome possibilities though like running postgres database or serving simple web server.
This one feature means that I own all of the data and don't have to manage any [cloud](https://aws.amazon.com/message/41926/) [services](https://www.theguardian.com/technology/2020/dec/14/google-suffers-worldwide-outage-with-gmail-youtube-and-other-services-down). One small drawback is that Raspberry Pi is missing analog inputs, this could be fixed by adding ADC.

Complete system should measure soil humidity and based on parametrization turn on water pump for given period of time.
Plants, especially herbs prefer fixed time of watering, preferably in the morning and not all like the same amount of water.
Each sensor reading and watering should be saved in database. Separate service should be able to serve them as dashboard via web page.

The final wiring and hardware components are the same as in million of other projects out there so I won't bore you with details ([one](https://medium.com/going-fullstack/watering-plants-with-a-raspberry-pi-36eac51b8d23), [second](https://www.hackster.io/ben-eagan/raspberry-pi-automated-plant-watering-with-website-8af2dc) or [next one](https://dev.to/alanconstantino/water-your-plant-using-a-raspberry-pi-and-python-2ddb)). Here is how I have wired it (I'm sure there is a lot of room for improvement):

{{< figure src="/images/rustypi/watering/watering_bb.png" class="img-lg">}}

I have used additional NPN transistor to turn on the power on the sensors only when needed following priceless [guidelines](https://raspberrypi.stackexchange.com/questions/68133/is-soil-moisture-sensor-corrosion-normal) and water level sensor protected by epoxy resin to detect when my water container is getting empty.

## Lets get shwifty 🦀

