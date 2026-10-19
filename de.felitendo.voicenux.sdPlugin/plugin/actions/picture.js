import { Action } from "./base.js";

// Keys that show pictures of voices or sounds. The picture comes from Voicenux, once per name.
export class PictureAction extends Action {
	// Loads the picture and draws again when it is there.
	picture(icon) {
		if (!icon) return null;
		if (this.pictureName !== icon) {
			this.pictureName = icon;
			this.png = null;
			this.client.icon(icon).then((png) => {
				if (this.pictureName !== icon || !png) return;
				this.png = png;
				if (this.online) this.safe(() => this.draw());
			});
		}
		return this.png;
	}

	onDisconnected() {
		// Pictures come again from the next Voicenux.
		this.pictureName = null;
		this.png = null;
	}
}
