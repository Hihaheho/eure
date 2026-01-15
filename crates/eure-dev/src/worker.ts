export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // Match /v{semver}/... paths
    const versionMatch = path.match(
      /^\/v(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?)\/(.*)/
    );
    if (versionMatch) {
      // CORS headers only for versioned schema paths
      const corsHeaders = {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type",
      };

      // Handle preflight OPTIONS request for versioned paths
      if (request.method === "OPTIONS") {
        return new Response(null, {
          headers: corsHeaders,
        });
      }

      const [, version, subPath] = versionMatch;
      const githubUrl = `https://raw.githubusercontent.com/Hihaheho/eure/v${version}/assets/${subPath}`;
      const response = await fetch(githubUrl);
      if (!response.ok) {
        return new Response("Not found", {
          status: 404,
          headers: corsHeaders,
        });
      }
      return new Response(response.body, {
        headers: {
          "Content-Type": "text/plain; charset=utf-8",
          ...corsHeaders,
        },
      });
    }

    // Fallback to static assets (no CORS headers)
    return env.ASSETS.fetch(request);
  },
};
