import { http, HttpResponse } from 'msw';

export const handlers = [
  // And here's a request handler with MSW
  // for the same "GET /user" request that
  // responds with a mock JSON response.
  http.get('/user', ({ request }) => {
    return HttpResponse.json({ name: 'John' });
  }),
];
