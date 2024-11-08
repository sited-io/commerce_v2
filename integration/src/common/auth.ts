import _ from "lodash";

export class Auth {
  private readonly authUrl;
  private readonly clientId: string;
  private readonly clientSecret: string;
  private headerValue = "";

  constructor(authUrl: string, clientId: string, clientSecret: string) {
    this.authUrl = authUrl;
    this.clientId = clientId;
    this.clientSecret = clientSecret;
  }

  async withAuthHeader() {
    if (_.isEmpty(this.headerValue)) {
      await this.getAccessToken();
    }
    return {
      headers: {
        authorization: this.headerValue,
      },
    };
  }

  async getAccessToken() {
    const res = await fetch(this.authUrl, {
      method: "POST",
      headers: {
        "content-type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        grant_type: "client_credentials",
        scope: "openid profile",
        client_id: this.clientId,
        client_secret: this.clientSecret,
      }),
    });
    const body = await res.json();
    this.headerValue = `Bearer ${body.access_token}`;
  }
}
