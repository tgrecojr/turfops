import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router";

const NAV_ITEMS = [
	{ to: "/", label: "Dashboard" },
	{ to: "/applications", label: "Applications" },
	{ to: "/inventory", label: "Inventory" },
	// @spec PLANT-UI-001
	{ to: "/landscape", label: "Landscape" },
	{ to: "/calendar", label: "Calendar" },
	{ to: "/environmental", label: "Environmental" },
	{ to: "/recommendations", label: "Recommendations" },
	{ to: "/disease-risk", label: "Disease Risk" },
	{ to: "/timing", label: "Seed & Pre-Em Timing" },
	{ to: "/soil-tests", label: "Soil Tests" },
	{ to: "/seasonal-plan", label: "Seasonal Plan" },
	{ to: "/settings", label: "Settings" },
];

/** Below this width the 220 px sidebar would crowd the page out. */
const NARROW_QUERY = "(max-width: 760px)";

/** Whether the viewport is phone-width, following resizes and rotation. */
function useNarrow(): boolean {
	const [narrow, setNarrow] = useState(
		() =>
			typeof window !== "undefined" && window.matchMedia(NARROW_QUERY).matches,
	);
	useEffect(() => {
		const query = window.matchMedia(NARROW_QUERY);
		const onChange = () => setNarrow(query.matches);
		query.addEventListener("change", onChange);
		return () => query.removeEventListener("change", onChange);
	}, []);
	return narrow;
}

// @spec UI-NAV-003
export default function Layout() {
	const narrow = useNarrow();
	const [menuOpen, setMenuOpen] = useState(false);
	const { pathname } = useLocation();
	// Going to a page closes the menu.
	useEffect(() => {
		void pathname;
		setMenuOpen(false);
	}, [pathname]);

	const links = (
		<ul id="main-nav-links" style={styles.navList}>
			{NAV_ITEMS.map((item) => (
				<li key={item.to}>
					<NavLink
						to={item.to}
						end={item.to === "/"}
						style={({ isActive }) => ({
							...styles.navLink,
							backgroundColor: isActive ? "#2d3748" : "transparent",
							color: isActive ? "#68d391" : "#cbd5e0",
							fontWeight: isActive ? 700 : 400,
						})}
					>
						{item.label}
					</NavLink>
				</li>
			))}
		</ul>
	);

	if (narrow) {
		return (
			<div style={{ minHeight: "100vh" }}>
				<nav style={styles.topBar} aria-label="Main navigation">
					<div style={styles.topRow}>
						<span style={{ fontSize: "1.2rem", fontWeight: 700 }}>TurfOps</span>
						<button
							type="button"
							style={styles.menuButton}
							aria-expanded={menuOpen}
							aria-controls="main-nav-links"
							onClick={() => setMenuOpen(!menuOpen)}
						>
							{menuOpen ? "✕ Close" : "☰ Menu"}
						</button>
					</div>
					{menuOpen && links}
				</nav>
				<main style={{ ...styles.content, padding: "1rem" }}>
					<Outlet />
				</main>
			</div>
		);
	}

	return (
		<div style={{ display: "flex", minHeight: "100vh" }}>
			<nav style={styles.sidebar} aria-label="Main navigation">
				<div style={styles.logo}>
					<span style={{ fontSize: "1.4rem" }}>TurfOps</span>
				</div>
				{links}
			</nav>
			<main style={styles.content}>
				<Outlet />
			</main>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	sidebar: {
		width: 220,
		backgroundColor: "#1a202c",
		color: "#fff",
		display: "flex",
		flexDirection: "column",
		flexShrink: 0,
	},
	logo: {
		padding: "1.2rem 1rem",
		borderBottom: "1px solid #2d3748",
		fontWeight: 700,
	},
	navList: {
		listStyle: "none",
		margin: 0,
		padding: "0.5rem 0",
	},
	navLink: {
		display: "block",
		padding: "0.6rem 1rem",
		textDecoration: "none",
		fontSize: "0.9rem",
		borderRadius: 4,
		margin: "2px 6px",
		transition: "background-color 0.15s",
	},
	topBar: {
		backgroundColor: "#1a202c",
		color: "#fff",
		position: "sticky",
		top: 0,
		zIndex: 10,
	},
	topRow: {
		display: "flex",
		alignItems: "center",
		justifyContent: "space-between",
		padding: "0.6rem 1rem",
	},
	menuButton: {
		background: "transparent",
		color: "#fff",
		border: "1px solid #4a5568",
		borderRadius: 6,
		padding: "0.35rem 0.7rem",
		fontSize: "0.9rem",
		cursor: "pointer",
	},
	content: {
		flex: 1,
		minWidth: 0,
		padding: "1.5rem 2rem",
		backgroundColor: "#f7fafc",
		overflowY: "auto" as const,
	},
};
