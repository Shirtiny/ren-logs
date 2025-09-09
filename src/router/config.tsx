import type { RouteObject } from 'react-router';
import type { INavItem } from './type';
import { Navigate } from 'react-router';
import RootLayout from '@/layouts/Root/index';
import ErrorPage from '@/pages/Error';
import NotFoundPage from '@/pages/404';

const navItems: INavItem[] = [
  {
    path: 'live',
    lazy: async () => import('@/pages/Welcome'),
    icon: null,
  },
  {
    path: 'adilraid',
    lazy: async () => import('@/pages/Edit'),
    icon: null,
  },
  {
    path: 'logs',
    lazy: async () => import('@/pages/Test'),
    icon: null,
  },
];

const v1: RouteObject[] = [
  {
    path: '/',
    errorElement: <ErrorPage />,
    element: <RootLayout />,
    children: [
      {
        index: true,
        element: <Navigate to={navItems[0].path} />,
      },
      ...navItems,
    ],
  },
  { path: '*', element: <NotFoundPage /> },
];

const routerConfig = { v1, navItems };

export default routerConfig;
