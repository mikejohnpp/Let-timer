import React from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { useMotionTokens, useMotionTransition } from '../motion';

export interface ViewHostProps {
  activeTab: string;
  children: React.ReactNode;
  className?: string;
}

export const ViewHost: React.FC<ViewHostProps> = ({ activeTab, children, className }) => {
  const tokens = useMotionTokens();
  const enterTransition = useMotionTransition('base');
  const exitTransition = useMotionTransition('base', { exit: true });

  const isReduced = tokens.isReduced;
  const travelY = isReduced ? 0 : tokens.travelS;

  return (
    <div className={className} data-testid="view-host">
      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={activeTab}
          data-testid={`view-tab-${activeTab}`}
          initial={{ opacity: 0, y: travelY }}
          animate={{ opacity: 1, y: 0, transition: enterTransition }}
          exit={{ opacity: 0, y: isReduced ? 0 : -travelY, transition: exitTransition }}
          className="h-full"
        >
          {children}
        </motion.div>
      </AnimatePresence>
    </div>
  );
};

