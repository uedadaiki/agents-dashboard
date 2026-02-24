const API_PORT = import.meta.env.VITE_API_PORT || "3001";

export const API_BASE = `http://${window.location.hostname}:${API_PORT}`;
export const WS_URL = `ws://${window.location.hostname}:${API_PORT}/ws`;
