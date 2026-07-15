import http from 'node:http';

const port = Number(process.argv[2] || 4179);

function page(body) {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Recreate access fixture</title>
    <style>
      body { font: 16px system-ui; margin: 0; }
      main { margin: 40px auto; max-width: 720px; padding: 32px; }
      input, button { display: block; margin-top: 16px; padding: 10px; }
    </style>
  </head>
  <body>${body}</body>
</html>`;
}

const server = http.createServer((request, response) => {
  if (request.url === '/session' && request.method === 'POST') {
    response.writeHead(303, {
      location: '/private',
      'set-cookie': 'recreate_session=ready; Path=/; HttpOnly; SameSite=Lax',
    });
    response.end();
    return;
  }

  if (request.url === '/private') {
    if (request.headers.cookie?.includes('recreate_session=ready')) {
      response.writeHead(200, { 'content-type': 'text/html' });
      response.end(page(`<main role="main">
        <h1>Private workspace</h1>
        <p>${'Captured application content. '.repeat(30)}</p>
        <button>Primary action</button>
      </main>`));
      return;
    }
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end(page(`<main>
      <form action="/session" method="post">
        <label>Password <input name="password" type="password"></label>
        <button type="submit">Continue</button>
      </form>
    </main>`));
    return;
  }

  if (request.url === '/text-only') {
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end(page(`<main role="main">
      <h1>Login and sign in are documentation terms</h1>
      <p>${'This public page explains account terminology. '.repeat(30)}</p>
      <button>Read more</button>
    </main>`));
    return;
  }

  response.writeHead(404);
  response.end('Not found');
});

server.listen(port, '127.0.0.1', () => {
  console.log(`access fixture listening on ${port}`);
});
