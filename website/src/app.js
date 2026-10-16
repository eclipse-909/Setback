import { xsml, router } from "xsml/xsml.js";

function index() {}
function notFound() {}

xsml(
	router("/not-found", {
		"/": index,
		"/not-found": notFound,
	})
);