import asyncio
import random
from asyncua import ua, Server

async def main():
    server = Server()
    await server.init()
    server.set_endpoint("opc.tcp://0.0.0.0:4840/freeopcua/server/")
    server.set_server_name("Dummy OPC UA Server")

    uri = "http://forgeio/dummy/"
    idx = await server.register_namespace(uri)
    print(f"Dummy OPC UA namespace index: {idx}")
    objects = server.nodes.objects

    temperature = await objects.add_variable(idx, "Temperature", 20.0)
    pressure = await objects.add_variable(idx, "Pressure", 1.0)
    counter = await objects.add_variable(idx, "Counter", 0)

    await temperature.set_writable()
    await pressure.set_writable()
    await counter.set_writable()

    async with server:
        while True:
            await temperature.write_value(random.uniform(15.0, 25.0))
            await pressure.write_value(random.uniform(0.8, 1.2))
            current = await counter.read_value()
            await counter.write_value(current + 1)
            await asyncio.sleep(1)

if __name__ == "__main__":
    asyncio.run(main())
