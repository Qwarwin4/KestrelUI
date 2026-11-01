import { useLayoutEffect, useRef } from "preact/hooks";
import { Outlet, useLocation } from "react-router";

import { Header, PageHeading } from "../header/index.tsx";
import { Navigation } from "../navigation/index.tsx";
import cs from "./index.module.css";

export function Layout() {
  const location = useLocation();
  const contentRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    // Scroll to the top of the page when the route changes
    contentRef.current?.scrollTo({ top: 0, left: 0, behavior: "instant" });
  }, [location.pathname]);

  return (
    <div className={cs.layout}>
      <Navigation />
      <Header />
      <div ref={contentRef} className={cs.content}>
        {/* keyed so every page change replays the enter animation */}
        <div key={location.pathname + location.search} className={cs.page}>
          <PageHeading />
          <Outlet />
        </div>
      </div>
    </div>
  );
}
