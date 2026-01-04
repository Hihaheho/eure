export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // Match /v{semver}/... paths
    const versionMatch = path.match(
      /^\/v(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?)\/(.*)/
    );
    if (versionMatch) {
      const [, version, subPath] = versionMatch;
      const githubUrl = `https://raw.githubusercontent.com/Hihaheho/eure/v${version}/assets/${subPath}`;
      const response = await fetch(githubUrl);
      if (!response.ok) {
        return new Response("Not found", { status: 404 });
      }
      return new Response(response.body, {
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    // Fallback to static assets
    return env.ASSETS.fetch(request);
  },
};
