const server = Bun.serve({
  port: 2212,
  hostname: "0.0.0.0",
  routes: {
    "/api/post": {
      POST: async (request) => {
        const body = await request.text();
        console.log("Recieved:");
        console.log(`  ${body}`);
        return new Response();
      },
    },
  },
});

console.log(`Server running on ${server.url}`);
