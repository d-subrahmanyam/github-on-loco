import { configureStore } from '@reduxjs/toolkit'
import createSagaMiddleware from 'redux-saga'
import browseReducer from './browseSlice'
import repositoriesReducer from './repositoriesSlice'
import rootSaga from './sagas'

const sagaMiddleware = createSagaMiddleware()

export const store = configureStore({
  reducer: {
    repositories: repositoriesReducer,
    browse: browseReducer,
  },
  // Saga owns side effects; the default async thunk middleware is not needed.
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({ thunk: false }).concat(sagaMiddleware),
})

sagaMiddleware.run(rootSaga)

export type RootState = ReturnType<typeof store.getState>
export type AppDispatch = typeof store.dispatch
